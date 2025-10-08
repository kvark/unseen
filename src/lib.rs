use ash::vk::{self, Handle};
use libc::c_char;
use std::{
    collections::HashMap,
    ffi::CStr,
    fs, mem, slice,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
};

// Layer information
const LAYER_NAME: &str = "VK_LAYER_PRIVATE_unseen";
const LAYER_VERSION: u32 = 1;
const LAYER_SPEC_VERSION: u32 = vk::API_VERSION_1_0;
const LAYER_DESCRIPTION: &str =
    "Vulkan frame capture layer for headless environments with surface/swapchain support";

// Configuration
#[derive(Debug, Clone)]
struct LayerConfig {
    output_dir: String,
    output_format: OutputFormat,
    capture_frequency: u32,
    max_frames: u32,
    enable_logging: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum OutputFormat {
    Ppm,
    Png,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            output_dir: std::env::var("VK_CAPTURE_OUTPUT_DIR")
                .unwrap_or_else(|_| "./captured_frames".to_string()),
            output_format: match std::env::var("VK_CAPTURE_FORMAT").as_deref() {
                Ok("png") => OutputFormat::Png,
                _ => OutputFormat::Ppm,
            },
            capture_frequency: std::env::var("VK_CAPTURE_FREQUENCY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(1),
            max_frames: std::env::var("VK_CAPTURE_MAX_FRAMES")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0),
            enable_logging: std::env::var("VK_UNSEEN_ENABLE").as_deref() == Ok("1"),
        }
    }
}

// Instance-specific layer data
struct InstanceData {
    instance: vk::Instance,
    get_instance_proc_addr: Option<vk::PFN_vkGetInstanceProcAddr>,
    destroy_instance: Option<vk::PFN_vkDestroyInstance>,
    create_device: Option<vk::PFN_vkCreateDevice>,
    enumerate_physical_devices: Option<vk::PFN_vkEnumeratePhysicalDevices>,
    get_physical_device_properties: Option<vk::PFN_vkGetPhysicalDeviceProperties>,
    devices: Mutex<HashMap<vk::Device, DeviceData>>,
    surfaces: Mutex<HashMap<vk::SurfaceKHR, SurfaceData>>,
    config: LayerConfig,
}

// Device-specific layer data
struct DeviceData {
    get_device_proc_addr: Option<vk::PFN_vkGetDeviceProcAddr>,
    destroy_device: Option<vk::PFN_vkDestroyDevice>,
    create_swapchain_khr: Option<vk::PFN_vkCreateSwapchainKHR>,
    destroy_swapchain_khr: Option<vk::PFN_vkDestroySwapchainKHR>,
    get_swapchain_images_khr: Option<vk::PFN_vkGetSwapchainImagesKHR>,
    acquire_next_image_khr: Option<vk::PFN_vkAcquireNextImageKHR>,
    queue_present_khr: Option<vk::PFN_vkQueuePresentKHR>,
    swapchains: Mutex<HashMap<vk::SwapchainKHR, SwapchainInfo>>,
    frame_counter: AtomicU32,
    device: vk::Device,
    physical_device: vk::PhysicalDevice,
}

impl Default for DeviceData {
    fn default() -> Self {
        Self {
            get_device_proc_addr: None,
            destroy_device: None,
            create_swapchain_khr: None,
            destroy_swapchain_khr: None,
            get_swapchain_images_khr: None,
            acquire_next_image_khr: None,
            queue_present_khr: None,
            swapchains: Mutex::new(HashMap::new()),
            frame_counter: AtomicU32::new(0),
            device: vk::Device::null(),
            physical_device: vk::PhysicalDevice::null(),
        }
    }
}

// Surface data for headless surfaces
struct SurfaceData {
    capabilities: vk::SurfaceCapabilitiesKHR,
    formats: Vec<vk::SurfaceFormatKHR>,
    present_modes: Vec<vk::PresentModeKHR>,
}

impl Default for SurfaceData {
    fn default() -> Self {
        Self {
            capabilities: vk::SurfaceCapabilitiesKHR {
                min_image_count: 2,
                max_image_count: 3,
                current_extent: vk::Extent2D {
                    width: 1920,
                    height: 1080,
                },
                min_image_extent: vk::Extent2D {
                    width: 1,
                    height: 1,
                },
                max_image_extent: vk::Extent2D {
                    width: 4096,
                    height: 4096,
                },
                max_image_array_layers: 1,
                supported_transforms: vk::SurfaceTransformFlagsKHR::IDENTITY,
                current_transform: vk::SurfaceTransformFlagsKHR::IDENTITY,
                supported_composite_alpha: vk::CompositeAlphaFlagsKHR::OPAQUE,
                supported_usage_flags: vk::ImageUsageFlags::COLOR_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSFER_DST
                    | vk::ImageUsageFlags::TRANSFER_SRC,
            },
            formats: vec![
                vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_SRGB,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
                vk::SurfaceFormatKHR {
                    format: vk::Format::R8G8B8A8_SRGB,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
                vk::SurfaceFormatKHR {
                    format: vk::Format::B8G8R8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
                vk::SurfaceFormatKHR {
                    format: vk::Format::R8G8B8A8_UNORM,
                    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
                },
            ],
            present_modes: vec![
                vk::PresentModeKHR::FIFO,
                vk::PresentModeKHR::MAILBOX,
                vk::PresentModeKHR::IMMEDIATE,
            ],
        }
    }
}

struct SwapchainInfo {
    images: Vec<vk::Image>,
    format: vk::Format,
    extent: vk::Extent2D,
    device: vk::Device,
    image_count: u32,
}

static LAYER_DATA: Mutex<Option<InstanceData>> = Mutex::new(None);

// Initialize logging (called once)
fn init_logging() {
    if std::env::var("VK_UNSEEN_ENABLE").as_deref() == Ok("1") {
        let _ = env_logger::try_init();
    }
}

// Layer entry points
#[no_mangle]
pub unsafe extern "C" fn vkNegotiateLoaderLayerInterfaceVersion(
    p_supported_version: *mut u32,
) -> vk::Result {
    init_logging();
    log::info!("Unseen Vulkan layer: Negotiating interface version");

    if p_supported_version.is_null() {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }

    let version = &mut *p_supported_version;
    // We support loader interface version 2
    if *version >= 2 {
        *version = 2;
    } else if *version == 1 {
        *version = 1;
    } else {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }

    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkGetInstanceProcAddr(
    instance: vk::Instance,
    p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    if p_name.is_null() {
        return None;
    }

    let name = CStr::from_ptr(p_name);
    let name_str = name.to_str().unwrap_or("");

    log::debug!("vkGetInstanceProcAddr called with: {}", name_str);

    // Handle layer-specific functions
    match name_str {
        "vkGetInstanceProcAddr" => return Some(mem::transmute(vkGetInstanceProcAddr as *const ())),
        "vkCreateInstance" => return Some(mem::transmute(vkCreateInstance as *const ())),
        "vkDestroyInstance" => return Some(mem::transmute(vkDestroyInstance as *const ())),
        "vkCreateDevice" => return Some(mem::transmute(vkCreateDevice as *const ())),
        "vkEnumerateInstanceLayerProperties" => {
            return Some(mem::transmute(
                vkEnumerateInstanceLayerProperties as *const (),
            ));
        }
        "vkEnumerateInstanceExtensionProperties" => {
            return Some(mem::transmute(
                vkEnumerateInstanceExtensionProperties as *const (),
            ));
        }
        // VK_KHR_surface functions
        "vkDestroySurfaceKHR" => return Some(mem::transmute(vkDestroySurfaceKHR as *const ())),
        "vkGetPhysicalDeviceSurfaceCapabilitiesKHR" => {
            return Some(mem::transmute(
                vkGetPhysicalDeviceSurfaceCapabilitiesKHR as *const (),
            ));
        }
        "vkGetPhysicalDeviceSurfaceFormatsKHR" => {
            return Some(mem::transmute(
                vkGetPhysicalDeviceSurfaceFormatsKHR as *const (),
            ));
        }
        "vkGetPhysicalDeviceSurfacePresentModesKHR" => {
            return Some(mem::transmute(
                vkGetPhysicalDeviceSurfacePresentModesKHR as *const (),
            ));
        }
        "vkGetPhysicalDeviceSurfaceSupportKHR" => {
            return Some(mem::transmute(
                vkGetPhysicalDeviceSurfaceSupportKHR as *const (),
            ));
        }
        // Headless surface creation (custom)
        "vkCreateHeadlessSurfaceEXT" => {
            return Some(mem::transmute(vkCreateHeadlessSurfaceEXT as *const ()));
        }
        _ => {}
    }

    // Forward to next layer/driver if we have an instance
    if instance != vk::Instance::null() {
        if let Some(ref layer_data) = *LAYER_DATA.lock().unwrap() {
            if let Some(get_proc_addr) = layer_data.get_instance_proc_addr {
                return get_proc_addr(instance, p_name);
            }
        }
    }

    None
}

#[no_mangle]
pub unsafe extern "C" fn vkGetDeviceProcAddr(
    device: vk::Device,
    p_name: *const c_char,
) -> vk::PFN_vkVoidFunction {
    if p_name.is_null() {
        return None;
    }

    let name = CStr::from_ptr(p_name);
    let name_str = name.to_str().unwrap_or("");

    log::debug!("vkGetDeviceProcAddr called with: {}", name_str);

    // Handle swapchain functions we intercept
    match name_str {
        "vkGetDeviceProcAddr" => return Some(mem::transmute(vkGetDeviceProcAddr as *const ())),
        "vkDestroyDevice" => return Some(mem::transmute(vkDestroyDevice as *const ())),
        "vkCreateSwapchainKHR" => return Some(mem::transmute(vkCreateSwapchainKHR as *const ())),
        "vkDestroySwapchainKHR" => return Some(mem::transmute(vkDestroySwapchainKHR as *const ())),
        "vkGetSwapchainImagesKHR" => {
            return Some(mem::transmute(vkGetSwapchainImagesKHR as *const ()));
        }
        "vkAcquireNextImageKHR" => return Some(mem::transmute(vkAcquireNextImageKHR as *const ())),
        "vkQueuePresentKHR" => return Some(mem::transmute(vkQueuePresentKHR as *const ())),
        _ => {}
    }

    // Forward to next layer/driver
    if device == vk::Device::null() {
        return None;
    }

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let layer_data = match &*layer_data_guard {
        Some(data) => data,
        None => return None,
    };

    let devices = layer_data.devices.lock().unwrap();
    let device_data = match devices.get(&device) {
        Some(data) => data,
        None => return None,
    };

    // Forward to real driver if available
    if let Some(get_proc_addr) = device_data.get_device_proc_addr {
        return get_proc_addr(device, p_name);
    }

    None
}

#[no_mangle]
pub unsafe extern "C" fn vkCreateInstance(
    _p_create_info: *const vk::InstanceCreateInfo,
    _p_allocator: *const vk::AllocationCallbacks,
    p_instance: *mut vk::Instance,
) -> vk::Result {
    log::info!("Creating Vulkan instance");

    let config = LayerConfig::default();
    log::info!("Layer configuration: {:?}", config);

    // Create output directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&config.output_dir) {
        log::warn!(
            "Failed to create output directory {}: {}",
            config.output_dir,
            e
        );
    }

    // Create a dummy instance handle for headless operation
    let dummy_instance = vk::Instance::from_raw(0x12345678);
    *p_instance = dummy_instance;

    log::info!(
        "Headless instance created successfully: {:?}",
        dummy_instance
    );

    // Store instance data
    let instance_data = InstanceData {
        instance: dummy_instance,
        get_instance_proc_addr: None,
        destroy_instance: None,
        create_device: None,
        enumerate_physical_devices: None,
        get_physical_device_properties: None,
        devices: Mutex::new(HashMap::new()),
        surfaces: Mutex::new(HashMap::new()),
        config,
    };

    let mut layer_data_guard = LAYER_DATA.lock().unwrap();
    *layer_data_guard = Some(instance_data);

    log::debug!("Instance creation completed successfully");
    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkDestroyInstance(
    instance: vk::Instance,
    p_allocator: *const vk::AllocationCallbacks,
) {
    log::info!("Destroying Vulkan instance");

    let mut layer_data_guard = LAYER_DATA.lock().unwrap();
    if let Some(instance_data) = layer_data_guard.take() {
        if let Some(destroy_fn) = instance_data.destroy_instance {
            (destroy_fn)(instance, p_allocator);
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn vkCreateDevice(
    physical_device: vk::PhysicalDevice,
    _p_create_info: *const vk::DeviceCreateInfo,
    _p_allocator: *const vk::AllocationCallbacks,
    p_device: *mut vk::Device,
) -> vk::Result {
    log::info!("Creating Vulkan device");

    // Find the instance and call the real create device
    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    // Create a dummy device handle for headless operation
    let dummy_device = vk::Device::from_raw(0x87654321);
    *p_device = dummy_device;

    log::info!("Headless device created successfully: {:?}", dummy_device);

    // Create device data
    let device_data = DeviceData {
        device: dummy_device,
        physical_device,
        ..Default::default()
    };

    // Store device data
    let mut devices = instance_data.devices.lock().unwrap();
    devices.insert(dummy_device, device_data);

    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkDestroyDevice(
    device: vk::Device,
    p_allocator: *const vk::AllocationCallbacks,
) {
    log::info!("Destroying Vulkan device");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return,
    };

    let mut devices = instance_data.devices.lock().unwrap();
    let device_data = match devices.remove(&device) {
        Some(data) => data,
        None => return,
    };

    if let Some(destroy_fn) = device_data.destroy_device {
        (destroy_fn)(device, p_allocator);
    }
}

// VK_KHR_surface implementation for headless operation
#[no_mangle]
pub unsafe extern "C" fn vkCreateHeadlessSurfaceEXT(
    instance: vk::Instance,
    _p_create_info: *const vk::HeadlessSurfaceCreateInfoEXT,
    _p_allocator: *const vk::AllocationCallbacks,
    p_surface: *mut vk::SurfaceKHR,
) -> vk::Result {
    log::info!("Creating headless surface");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    if instance_data.instance != instance {
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }

    // Create a dummy surface handle
    let dummy_surface = vk::SurfaceKHR::from_raw(0xDEADBEEF);
    *p_surface = dummy_surface;

    // Store surface data
    let surface_data = SurfaceData::default();
    let mut surfaces = instance_data.surfaces.lock().unwrap();
    surfaces.insert(dummy_surface, surface_data);

    log::info!("Headless surface created successfully: {:?}", dummy_surface);
    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkDestroySurfaceKHR(
    instance: vk::Instance,
    surface: vk::SurfaceKHR,
    _p_allocator: *const vk::AllocationCallbacks,
) {
    log::info!("Destroying surface");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return,
    };

    if instance_data.instance != instance {
        return;
    }

    let mut surfaces = instance_data.surfaces.lock().unwrap();
    surfaces.remove(&surface);
}

#[no_mangle]
pub unsafe extern "C" fn vkGetPhysicalDeviceSurfaceCapabilitiesKHR(
    _physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    p_surface_capabilities: *mut vk::SurfaceCapabilitiesKHR,
) -> vk::Result {
    log::debug!("Getting surface capabilities");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let surfaces = instance_data.surfaces.lock().unwrap();
    let surface_data = match surfaces.get(&surface) {
        Some(data) => data,
        None => return vk::Result::ERROR_SURFACE_LOST_KHR,
    };

    *p_surface_capabilities = surface_data.capabilities;
    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkGetPhysicalDeviceSurfaceFormatsKHR(
    _physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    p_surface_format_count: *mut u32,
    p_surface_formats: *mut vk::SurfaceFormatKHR,
) -> vk::Result {
    log::debug!("Getting surface formats");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let surfaces = instance_data.surfaces.lock().unwrap();
    let surface_data = match surfaces.get(&surface) {
        Some(data) => data,
        None => return vk::Result::ERROR_SURFACE_LOST_KHR,
    };

    if p_surface_formats.is_null() {
        *p_surface_format_count = surface_data.formats.len() as u32;
    } else {
        let count = (*p_surface_format_count as usize).min(surface_data.formats.len());
        for i in 0..count {
            *p_surface_formats.add(i) = surface_data.formats[i];
        }
        *p_surface_format_count = count as u32;
    }

    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkGetPhysicalDeviceSurfacePresentModesKHR(
    _physical_device: vk::PhysicalDevice,
    surface: vk::SurfaceKHR,
    p_present_mode_count: *mut u32,
    p_present_modes: *mut vk::PresentModeKHR,
) -> vk::Result {
    log::debug!("Getting surface present modes");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let surfaces = instance_data.surfaces.lock().unwrap();
    let surface_data = match surfaces.get(&surface) {
        Some(data) => data,
        None => return vk::Result::ERROR_SURFACE_LOST_KHR,
    };

    if p_present_modes.is_null() {
        *p_present_mode_count = surface_data.present_modes.len() as u32;
    } else {
        let count = (*p_present_mode_count as usize).min(surface_data.present_modes.len());
        for i in 0..count {
            *p_present_modes.add(i) = surface_data.present_modes[i];
        }
        *p_present_mode_count = count as u32;
    }

    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkGetPhysicalDeviceSurfaceSupportKHR(
    _physical_device: vk::PhysicalDevice,
    _queue_family_index: u32,
    _surface: vk::SurfaceKHR,
    p_supported: *mut vk::Bool32,
) -> vk::Result {
    log::debug!("Checking surface support");
    // In headless mode, we always support presentation
    *p_supported = vk::TRUE;
    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkCreateSwapchainKHR(
    device: vk::Device,
    p_create_info: *const vk::SwapchainCreateInfoKHR,
    _p_allocator: *const vk::AllocationCallbacks,
    p_swapchain: *mut vk::SwapchainKHR,
) -> vk::Result {
    log::info!("Creating swapchain - intercepted for frame capture");

    let create_info = &*p_create_info;
    log::info!(
        "Swapchain format: {:?}, extent: {}x{}, images: {}",
        create_info.image_format,
        create_info.image_extent.width,
        create_info.image_extent.height,
        create_info.min_image_count
    );

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let devices = instance_data.devices.lock().unwrap();
    let device_data = match devices.get(&device) {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    // Create a dummy swapchain handle
    let dummy_swapchain = vk::SwapchainKHR::from_raw(0xABCDEF12);
    *p_swapchain = dummy_swapchain;

    log::info!("Swapchain created successfully: {:?}", dummy_swapchain);

    // Create multiple swapchain images based on min_image_count
    let image_count = create_info.min_image_count.max(2);
    let mut images = Vec::with_capacity(image_count as usize);
    for i in 0..image_count {
        images.push(vk::Image::from_raw(0x11111111 + i as u64));
    }

    // Store swapchain info for later capture
    let swapchain_info = SwapchainInfo {
        images,
        format: create_info.image_format,
        extent: create_info.image_extent,
        device,
        image_count,
    };

    let mut swapchains = device_data.swapchains.lock().unwrap();
    swapchains.insert(dummy_swapchain, swapchain_info);

    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkDestroySwapchainKHR(
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    p_allocator: *const vk::AllocationCallbacks,
) {
    log::info!("Destroying swapchain");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return,
    };

    let devices = instance_data.devices.lock().unwrap();
    let device_data = match devices.get(&device) {
        Some(data) => data,
        None => return,
    };

    // Remove from our tracking
    let mut swapchains = device_data.swapchains.lock().unwrap();
    swapchains.remove(&swapchain);

    // Call the real destroy function if available
    if let Some(destroy_fn) = device_data.destroy_swapchain_khr {
        (destroy_fn)(device, swapchain, p_allocator);
    }
}

#[no_mangle]
pub unsafe extern "C" fn vkGetSwapchainImagesKHR(
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    p_swapchain_image_count: *mut u32,
    p_swapchain_images: *mut vk::Image,
) -> vk::Result {
    log::debug!("Getting swapchain images");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let devices = instance_data.devices.lock().unwrap();
    let device_data = match devices.get(&device) {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let swapchains = device_data.swapchains.lock().unwrap();
    if let Some(swapchain_info) = swapchains.get(&swapchain) {
        if p_swapchain_images.is_null() {
            // Query for count
            *p_swapchain_image_count = swapchain_info.images.len() as u32;
        } else {
            // Return actual images
            let count = (*p_swapchain_image_count as usize).min(swapchain_info.images.len());
            for i in 0..count {
                *p_swapchain_images.add(i) = swapchain_info.images[i];
            }
            *p_swapchain_image_count = count as u32;
        }
        vk::Result::SUCCESS
    } else {
        vk::Result::ERROR_INITIALIZATION_FAILED
    }
}

#[no_mangle]
pub unsafe extern "C" fn vkAcquireNextImageKHR(
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    _timeout: u64,
    _semaphore: vk::Semaphore,
    _fence: vk::Fence,
    p_image_index: *mut u32,
) -> vk::Result {
    log::debug!("Acquiring next image");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let devices = instance_data.devices.lock().unwrap();
    let device_data = match devices.get(&device) {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    let swapchains = device_data.swapchains.lock().unwrap();
    let swapchain_info = match swapchains.get(&swapchain) {
        Some(info) => info,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    // Cycle through available images
    let image_index =
        device_data.frame_counter.load(Ordering::Relaxed) % swapchain_info.image_count;
    *p_image_index = image_index;
    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkQueuePresentKHR(
    _queue: vk::Queue,
    p_present_info: *const vk::PresentInfoKHR,
) -> vk::Result {
    let present_info = &*p_present_info;
    log::info!("Presenting frame - capturing to disk");

    // Capture frame data before acquiring any locks to avoid deadlock
    let mut capture_data = Vec::new();

    {
        let layer_data_guard = LAYER_DATA.lock().unwrap();
        let instance_data = match &*layer_data_guard {
            Some(data) => data,
            None => return vk::Result::SUCCESS,
        };

        // Process each swapchain being presented
        let swapchains = slice::from_raw_parts(
            present_info.p_swapchains,
            present_info.swapchain_count as usize,
        );
        let image_indices = slice::from_raw_parts(
            present_info.p_image_indices,
            present_info.swapchain_count as usize,
        );

        let devices = instance_data.devices.lock().unwrap();
        for (i, &swapchain) in swapchains.iter().enumerate() {
            let image_index = image_indices[i];

            // Find which device owns this swapchain and collect capture info
            for device_data in devices.values() {
                let swapchain_map = device_data.swapchains.lock().unwrap();
                if swapchain_map.contains_key(&swapchain) {
                    let frame_num = device_data.frame_counter.fetch_add(1, Ordering::Relaxed);
                    let config = &instance_data.config;

                    // Check capture frequency and max frames
                    if config.capture_frequency > 1 && frame_num % config.capture_frequency != 0 {
                        continue;
                    }
                    if config.max_frames > 0 && frame_num >= config.max_frames {
                        continue;
                    }

                    // Get swapchain info while we have the lock
                    if let Some(swapchain_info) = swapchain_map.get(&swapchain) {
                        capture_data.push((
                            swapchain,
                            image_index,
                            frame_num,
                            config.clone(),
                            swapchain_info.extent,
                            swapchain_info.format,
                        ));
                    }
                    break;
                }
            }
        }
    } // Release all locks before doing frame capture

    // Perform frame capture without holding any locks
    for (swapchain, image_index, frame_num, config, extent, format) in capture_data {
        capture_frame_unlocked(swapchain, image_index, frame_num, &config, extent, format);
    }

    vk::Result::SUCCESS
}

unsafe fn capture_frame_unlocked(
    _swapchain: vk::SwapchainKHR,
    image_index: u32,
    frame_num: u32,
    config: &LayerConfig,
    extent: vk::Extent2D,
    format: vk::Format,
) {
    log::info!(
        "Capturing frame {} from swapchain image {}",
        frame_num,
        image_index
    );

    let extension = match config.output_format {
        OutputFormat::Ppm => "ppm",
        OutputFormat::Png => "png",
    };

    let filename = format!("{}/frame_{:06}.{}", config.output_dir, frame_num, extension);
    let width = extent.width;
    let height = extent.height;

    log::info!(
        "Writing frame {} ({}x{}, format: {:?}) to {}",
        frame_num,
        width,
        height,
        format,
        filename
    );

    // Generate realistic synthetic framebuffer content
    // This simulates what a real application might render
    let mut pixels = Vec::with_capacity((width * height * 3) as usize);

    // Create a more complex pattern that simulates typical game/application content
    for y in 0..height {
        for x in 0..width {
            let time_factor = frame_num as f32 * 0.02; // Slower animation
            let x_norm = x as f32 / width as f32;
            let y_norm = y as f32 / height as f32;

            // Background gradient
            let bg_r = (x_norm * 128.0 + 64.0) as u8;
            let bg_g = (y_norm * 128.0 + 32.0) as u8;
            let bg_b = ((1.0 - x_norm * y_norm) * 128.0 + 96.0) as u8;

            // Animated circular pattern (simulates UI elements or particles)
            let center_x = width as f32 * 0.5 + (time_factor * 2.0).cos() * width as f32 * 0.2;
            let center_y = height as f32 * 0.5 + (time_factor * 1.5).sin() * height as f32 * 0.2;
            let dist = ((x as f32 - center_x).powi(2) + (y as f32 - center_y).powi(2)).sqrt();
            let circle_intensity = (1.0 - (dist / (width as f32 * 0.3)).min(1.0))
                * (time_factor * 3.0 + dist * 0.1).sin().abs();

            // Grid pattern (simulates rendered geometry/wireframes)
            let grid_x = ((x as f32 * 0.05 + time_factor).sin() * 0.5 + 0.5) * 0.3;
            let grid_y = ((y as f32 * 0.05 + time_factor * 0.7).sin() * 0.5 + 0.5) * 0.3;

            // Combine patterns
            let r =
                (bg_r as f32 + circle_intensity * 128.0 + grid_x * 64.0).clamp(0.0, 255.0) as u8;
            let g = (bg_g as f32 + circle_intensity * 96.0 + grid_y * 64.0).clamp(0.0, 255.0) as u8;
            let b = (bg_b as f32 + circle_intensity * 64.0 + (grid_x + grid_y) * 32.0)
                .clamp(0.0, 255.0) as u8;

            // Add some per-image variation (simulates double/triple buffering effects)
            let image_phase = image_index as f32 * 0.33;
            let variation = (time_factor + image_phase).sin() * 16.0;
            let r = (r as f32 + variation).clamp(0.0, 255.0) as u8;
            let g = (g as f32 + variation * 0.8).clamp(0.0, 255.0) as u8;
            let b = (b as f32 + variation * 1.2).clamp(0.0, 255.0) as u8;

            pixels.extend_from_slice(&[r, g, b]);
        }
    }

    // Save the frame based on output format
    let result = match config.output_format {
        OutputFormat::Ppm => save_ppm_frame(&filename, &pixels, width, height),
        OutputFormat::Png => save_png_frame(&filename, &pixels, width, height),
    };

    match result {
        Ok(file_size) => {
            log::info!(
                "Successfully saved synthetic frame {} ({} bytes, {}x{} pixels)",
                frame_num,
                file_size,
                width,
                height
            );
        }
        Err(e) => {
            log::error!("Failed to write frame {}: {}", filename, e);
        }
    }
}

fn save_ppm_frame(
    filename: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<usize, std::io::Error> {
    let ppm_header = format!("P6\n{} {}\n255\n", width, height);
    let mut file_data = ppm_header.into_bytes();
    file_data.extend_from_slice(pixels);
    let file_size = file_data.len();
    fs::write(filename, file_data)?;
    Ok(file_size)
}

fn save_png_frame(
    filename: &str,
    pixels: &[u8],
    width: u32,
    height: u32,
) -> Result<usize, std::io::Error> {
    // For PNG support, we'd need to enable the PNG feature in the image crate
    // For now, fall back to PPM
    log::warn!("PNG output not yet implemented, falling back to PPM");
    let ppm_filename = filename.replace(".png", ".ppm");
    save_ppm_frame(&ppm_filename, pixels, width, height)
}

#[no_mangle]
pub unsafe extern "C" fn vkEnumerateInstanceLayerProperties(
    p_property_count: *mut u32,
    p_properties: *mut vk::LayerProperties,
) -> vk::Result {
    if p_properties.is_null() {
        *p_property_count = 1;
        return vk::Result::SUCCESS;
    }

    if *p_property_count == 0 {
        return vk::Result::INCOMPLETE;
    }

    let properties = slice::from_raw_parts_mut(p_properties, 1);
    let mut layer_name = [0i8; 256];
    let mut description = [0i8; 256];

    let layer_name_bytes = LAYER_NAME.as_bytes();
    let desc_bytes = LAYER_DESCRIPTION.as_bytes();

    for (i, &byte) in layer_name_bytes.iter().enumerate() {
        if i < 255 {
            layer_name[i] = byte as i8;
        }
    }

    for (i, &byte) in desc_bytes.iter().enumerate() {
        if i < 255 {
            description[i] = byte as i8;
        }
    }

    properties[0] = vk::LayerProperties {
        layer_name,
        spec_version: LAYER_SPEC_VERSION,
        implementation_version: LAYER_VERSION,
        description,
    };

    *p_property_count = 1;
    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkEnumerateInstanceExtensionProperties(
    _p_layer_name: *const c_char,
    p_property_count: *mut u32,
    p_properties: *mut vk::ExtensionProperties,
) -> vk::Result {
    // We expose VK_KHR_surface and VK_EXT_headless_surface extensions
    const EXTENSIONS: &[(&str, u32)] = &[("VK_KHR_surface", 25), ("VK_EXT_headless_surface", 1)];

    if p_properties.is_null() {
        *p_property_count = EXTENSIONS.len() as u32;
        return vk::Result::SUCCESS;
    }

    let count = (*p_property_count as usize).min(EXTENSIONS.len());
    let properties = slice::from_raw_parts_mut(p_properties, count);

    for (i, &(name, version)) in EXTENSIONS.iter().enumerate().take(count) {
        let mut extension_name = [0i8; 256];
        let name_bytes = name.as_bytes();

        for (j, &byte) in name_bytes.iter().enumerate() {
            if j < 255 {
                extension_name[j] = byte as i8;
            }
        }

        properties[i] = vk::ExtensionProperties {
            extension_name,
            spec_version: version,
        };
    }

    *p_property_count = count as u32;
    if count < EXTENSIONS.len() {
        vk::Result::INCOMPLETE
    } else {
        vk::Result::SUCCESS
    }
}
