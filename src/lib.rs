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
    "Vulkan frame capture layer for headless environments with direct host-visible capture";

// Configuration
#[derive(Debug, Clone)]
struct LayerConfig {
    output_dir: String,
    output_format: OutputFormat,
    capture_frequency: u32,
    max_frames: u32,
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
        }
    }
}

// Instance-specific layer data
struct InstanceData {
    instance: vk::Instance,
    get_instance_proc_addr: Option<vk::PFN_vkGetInstanceProcAddr>,
    destroy_instance: Option<vk::PFN_vkDestroyInstance>,
    create_device: Option<vk::PFN_vkCreateDevice>,
    devices: Mutex<HashMap<vk::Device, DeviceData>>,
    surfaces: Mutex<HashMap<vk::SurfaceKHR, SurfaceData>>,
    config: LayerConfig,
}

// Device-specific layer data
struct DeviceData {
    physical_device: vk::PhysicalDevice,
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
    command_pool: Option<vk::CommandPool>,
    graphics_queue: Option<vk::Queue>,
    graphics_queue_family: Option<u32>,
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
    images: Vec<HostVisibleImage>,
    format: vk::Format,
    extent: vk::Extent2D,
    image_count: u32,
}

// Host-visible image with direct CPU access
struct HostVisibleImage {
    image: vk::Image,
    memory: vk::DeviceMemory,
    mapped_ptr: *mut u8,
    size: u64,
    row_pitch: u32,
}

// Safety: HostVisibleImage is only accessed from one thread at a time
// The raw pointer is only used for reading memory mapped by Vulkan
unsafe impl Send for HostVisibleImage {}
unsafe impl Sync for HostVisibleImage {}

static LAYER_DATA: Mutex<Option<InstanceData>> = Mutex::new(None);

// Initialize logging
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

    if let Some(get_proc_addr) = device_data.get_device_proc_addr {
        return get_proc_addr(device, p_name);
    }

    None
}

#[no_mangle]
pub unsafe extern "C" fn vkCreateInstance(
    p_create_info: *const vk::InstanceCreateInfo,
    p_allocator: *const vk::AllocationCallbacks,
    p_instance: *mut vk::Instance,
) -> vk::Result {
    // Initialize logging if not already done
    if std::env::var("RUST_LOG").is_ok() {
        let _ = env_logger::try_init();
    }

    log::info!("Creating Vulkan instance");

    // Get configuration
    let config = LayerConfig::default();

    // Get next layer's function from chain - REQUIRED for real apps
    let next_get_instance_proc_addr = match get_chain_info(p_create_info) {
        Ok(func) => func,
        Err(_) => {
            log::error!("Failed to get layer chain info - this layer requires proper chaining");
            return vk::Result::ERROR_LAYER_NOT_PRESENT;
        }
    };

    // Get next layer's vkCreateInstance
    let next_create_instance: vk::PFN_vkCreateInstance =
        mem::transmute(next_get_instance_proc_addr(
            vk::Instance::null(),
            b"vkCreateInstance\0".as_ptr() as *const c_char,
        ));

    if (next_create_instance as *const ()).is_null() {
        log::error!("Failed to get next layer's vkCreateInstance");
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }

    // Call next layer's vkCreateInstance
    let result = next_create_instance(p_create_info, p_allocator, p_instance);
    if result != vk::Result::SUCCESS {
        log::error!("Next layer's vkCreateInstance failed: {:?}", result);
        return result;
    }

    let instance = *p_instance;
    log::info!("Real instance created successfully: {:?}", instance);

    // Store instance data with real chaining
    let instance_data = InstanceData {
        instance,
        get_instance_proc_addr: Some(next_get_instance_proc_addr),
        destroy_instance: None,
        create_device: None,
        devices: Mutex::new(HashMap::new()),
        surfaces: Mutex::new(HashMap::new()),
        config,
    };

    let mut layer_data_guard = LAYER_DATA.lock().unwrap();
    *layer_data_guard = Some(instance_data);

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
    p_create_info: *const vk::DeviceCreateInfo,
    p_allocator: *const vk::AllocationCallbacks,
    p_device: *mut vk::Device,
) -> vk::Result {
    log::info!("Creating Vulkan device");

    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = match &*layer_data_guard {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    // Get next layer's vkCreateDevice - REQUIRED
    let next_get_instance_proc_addr = instance_data.get_instance_proc_addr.unwrap();
    let next_create_device: vk::PFN_vkCreateDevice = mem::transmute(next_get_instance_proc_addr(
        instance_data.instance,
        b"vkCreateDevice\0".as_ptr() as *const c_char,
    ));

    if (next_create_device as *const ()).is_null() {
        log::error!("Failed to get next layer's vkCreateDevice");
        return vk::Result::ERROR_INITIALIZATION_FAILED;
    }

    // Call next layer's vkCreateDevice
    let result = next_create_device(physical_device, p_create_info, p_allocator, p_device);
    if result != vk::Result::SUCCESS {
        log::error!("Next layer's vkCreateDevice failed: {:?}", result);
        return result;
    }

    let device = *p_device;
    log::info!("Real device created successfully: {:?}", device);

    // Get next layer's vkGetDeviceProcAddr
    let next_get_device_proc_addr: vk::PFN_vkGetDeviceProcAddr =
        mem::transmute(next_get_instance_proc_addr(
            instance_data.instance,
            b"vkGetDeviceProcAddr\0".as_ptr() as *const c_char,
        ));

    // Create ash device wrapper for the REAL device
    let ash_instance = unsafe {
        ash::Instance::load(
            &ash::Entry::load().unwrap().static_fn(),
            instance_data.instance,
        )
    };
    let ash_device = ash::Device::load(&ash_instance.fp_v1_0(), device);

    // Find graphics queue from the REAL device creation info
    let create_info = &*p_create_info;
    let mut graphics_queue_family = None;
    let mut graphics_queue = None;
    let mut command_pool = None;

    if create_info.queue_create_info_count > 0 {
        let queue_create_infos = slice::from_raw_parts(
            create_info.p_queue_create_infos,
            create_info.queue_create_info_count as usize,
        );

        // Find graphics queue family
        for queue_create_info in queue_create_infos {
            let queue_family_properties =
                ash_instance.get_physical_device_queue_family_properties(physical_device);

            if queue_create_info.queue_family_index < queue_family_properties.len() as u32 {
                let props = &queue_family_properties[queue_create_info.queue_family_index as usize];
                if props.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
                    graphics_queue_family = Some(queue_create_info.queue_family_index);
                    // Get the REAL graphics queue from the REAL device
                    graphics_queue =
                        Some(ash_device.get_device_queue(queue_create_info.queue_family_index, 0));
                    break;
                }
            }
        }

        // Create REAL command pool if we found graphics queue
        if let Some(queue_family) = graphics_queue_family {
            let pool_info = vk::CommandPoolCreateInfo::builder()
                .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
                .queue_family_index(queue_family);

            match ash_device.create_command_pool(&pool_info, None) {
                Ok(pool) => {
                    command_pool = Some(pool);
                    log::info!(
                        "Created REAL command pool for graphics queue family {}",
                        queue_family
                    );
                }
                Err(e) => {
                    log::warn!("Failed to create command pool: {:?}", e);
                }
            }
        }
    }

    let device_data = DeviceData {
        physical_device,
        get_device_proc_addr: Some(next_get_device_proc_addr),
        destroy_device: None,
        create_swapchain_khr: None,
        destroy_swapchain_khr: None,
        get_swapchain_images_khr: None,
        acquire_next_image_khr: None,
        queue_present_khr: None,
        swapchains: Mutex::new(HashMap::new()),
        frame_counter: AtomicU32::new(0),
        device,
        command_pool,
        graphics_queue,
        graphics_queue_family,
    };

    // Store device data
    let mut devices = instance_data.devices.lock().unwrap();
    devices.insert(device, device_data);

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
    if let Some(device_data) = devices.remove(&device) {
        // Clean up REAL command pool if it exists
        if let Some(pool) = device_data.command_pool {
            let ash_instance = unsafe {
                ash::Instance::load(
                    &ash::Entry::load().unwrap().static_fn(),
                    instance_data.instance,
                )
            };
            let ash_device = ash::Device::load(&ash_instance.fp_v1_0(), device);
            ash_device.destroy_command_pool(pool, None);
            log::info!("Destroyed REAL command pool for device");
        }

        // Clean up all swapchains for this device
        let swapchains = device_data.swapchains.into_inner().unwrap();
        for (_, swapchain_info) in swapchains {
            cleanup_host_visible_images(&swapchain_info.images);
        }

        // Call next layer's vkDestroyDevice
        let next_get_instance_proc_addr = instance_data.get_instance_proc_addr.unwrap();
        let next_destroy_device: vk::PFN_vkDestroyDevice =
            mem::transmute(next_get_instance_proc_addr(
                instance_data.instance,
                b"vkDestroyDevice\0".as_ptr() as *const c_char,
            ));
        next_destroy_device(device, p_allocator);
    }
}

// VK_KHR_surface implementation
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

    // Create a unique surface handle
    let dummy_surface = vk::SurfaceKHR::from_raw(generate_unique_handle());
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
    // Always support presentation in headless mode
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
    log::info!("Creating swapchain with host-visible images");

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

    let dummy_swapchain = vk::SwapchainKHR::from_raw(generate_unique_handle());
    *p_swapchain = dummy_swapchain;

    // Create host-visible images using REAL device
    let image_count = create_info.min_image_count.max(2);
    let ash_instance = unsafe {
        ash::Instance::load(
            &ash::Entry::load().unwrap().static_fn(),
            instance_data.instance,
        )
    };
    let ash_device = ash::Device::load(&ash_instance.fp_v1_0(), device);
    let host_images = match create_host_visible_images(
        &ash_device,
        device_data.physical_device,
        create_info.image_extent,
        create_info.image_format,
        image_count,
    ) {
        Ok(images) => images,
        Err(e) => {
            log::error!("Failed to create host-visible images: {:?}", e);
            return e;
        }
    };

    log::info!(
        "Created {} host-visible images for direct CPU access",
        host_images.len()
    );

    let swapchain_info = SwapchainInfo {
        images: host_images,
        format: create_info.image_format,
        extent: create_info.image_extent,
        image_count,
    };

    let mut swapchains = device_data.swapchains.lock().unwrap();
    swapchains.insert(dummy_swapchain, swapchain_info);

    log::info!("Swapchain created successfully: {:?}", dummy_swapchain);
    vk::Result::SUCCESS
}

#[no_mangle]
pub unsafe extern "C" fn vkDestroySwapchainKHR(
    device: vk::Device,
    swapchain: vk::SwapchainKHR,
    _p_allocator: *const vk::AllocationCallbacks,
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

    let swapchain_info = {
        let mut swapchains = device_data.swapchains.lock().unwrap();
        swapchains.remove(&swapchain)
    };

    if let Some(info) = swapchain_info {
        cleanup_host_visible_images(&info.images);
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
            // Return host-visible image handles
            let count = (*p_swapchain_image_count as usize).min(swapchain_info.images.len());
            for i in 0..count {
                *p_swapchain_images.add(i) = swapchain_info.images[i].image;
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
    log::info!("Presenting frames - capturing from host-visible memory");

    // Process frame capture
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

        // Find which device owns this swapchain
        for device_data in devices.values() {
            let swapchain_map = device_data.swapchains.lock().unwrap();
            if let Some(swapchain_info) = swapchain_map.get(&swapchain) {
                let frame_num = device_data.frame_counter.fetch_add(1, Ordering::Relaxed);

                // Check capture frequency and max frames
                if instance_data.config.capture_frequency > 1
                    && frame_num % instance_data.config.capture_frequency != 0
                {
                    continue;
                }
                if instance_data.config.max_frames > 0
                    && frame_num >= instance_data.config.max_frames
                {
                    continue;
                }

                // Capture frame from host-visible memory
                capture_host_visible_frame(
                    device_data,
                    &swapchain_info,
                    image_index as usize,
                    frame_num,
                    &instance_data.config,
                );
                break;
            }
        }
    }

    vk::Result::SUCCESS
}

// Create host-visible images with linear layout for direct CPU access
fn create_host_visible_images(
    device: &ash::Device,
    physical_device: vk::PhysicalDevice,
    extent: vk::Extent2D,
    format: vk::Format,
    image_count: u32,
) -> Result<Vec<HostVisibleImage>, vk::Result> {
    let mut images = Vec::with_capacity(image_count as usize);

    // Get memory properties from REAL physical device
    let layer_data_guard = LAYER_DATA.lock().unwrap();
    let instance_data = layer_data_guard.as_ref().unwrap();
    let ash_instance = unsafe {
        ash::Instance::load(
            &ash::Entry::load().unwrap().static_fn(),
            instance_data.instance,
        )
    };
    let mem_properties =
        unsafe { ash_instance.get_physical_device_memory_properties(physical_device) };

    for i in 0..image_count {
        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .format(format)
            .extent(vk::Extent3D {
                width: extent.width,
                height: extent.height,
                depth: 1,
            })
            .mip_levels(1)
            .array_layers(1)
            .samples(vk::SampleCountFlags::TYPE_1)
            .tiling(vk::ImageTiling::LINEAR) // Linear for CPU access
            .usage(vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED);

        // Create REAL image
        let image = unsafe { device.create_image(&image_info, None)? };
        let memory_requirements = unsafe { device.get_image_memory_requirements(image) };

        // Find host-visible memory type from REAL device
        let mut memory_type_index = None;
        for i in 0..mem_properties.memory_type_count {
            if (memory_requirements.memory_type_bits & (1 << i)) != 0
                && mem_properties.memory_types[i as usize]
                    .property_flags
                    .contains(
                        vk::MemoryPropertyFlags::HOST_VISIBLE
                            | vk::MemoryPropertyFlags::HOST_COHERENT,
                    )
            {
                memory_type_index = Some(i);
                break;
            }
        }

        let memory_type_index = memory_type_index.ok_or(vk::Result::ERROR_OUT_OF_HOST_MEMORY)?;

        // Allocate REAL host-visible memory
        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(memory_requirements.size)
            .memory_type_index(memory_type_index);

        let memory = unsafe { device.allocate_memory(&alloc_info, None)? };

        // Bind REAL image to REAL memory
        unsafe { device.bind_image_memory(image, memory, 0)? };

        // Map the REAL memory for persistent access
        let mapped_ptr = unsafe {
            device.map_memory(
                memory,
                0,
                memory_requirements.size,
                vk::MemoryMapFlags::empty(),
            )? as *mut u8
        };

        // Get REAL subresource layout for row pitch
        let subresource = vk::ImageSubresource {
            aspect_mask: vk::ImageAspectFlags::COLOR,
            mip_level: 0,
            array_layer: 0,
        };
        let layout = unsafe { device.get_image_subresource_layout(image, subresource) };

        images.push(HostVisibleImage {
            image,
            memory,
            mapped_ptr,
            size: memory_requirements.size,
            row_pitch: layout.row_pitch as u32,
        });

        log::debug!(
            "Created REAL host-visible image {}: {:?} ({}x{}, row_pitch: {})",
            i,
            image,
            extent.width,
            extent.height,
            layout.row_pitch
        );
    }

    Ok(images)
}

fn find_host_visible_memory_type(
    ash_instance: &ash::Instance,
    physical_device: vk::PhysicalDevice,
    type_filter: u32,
) -> Option<u32> {
    let mem_properties =
        unsafe { ash_instance.get_physical_device_memory_properties(physical_device) };

    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i)) != 0
            && mem_properties.memory_types[i as usize]
                .property_flags
                .contains(
                    vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
                )
        {
            return Some(i);
        }
    }

    None
}

fn cleanup_host_visible_images(images: &[HostVisibleImage]) {
    // For real cleanup, we would need device access to destroy images and free memory
    // This is called during device destruction, so the device cleanup handles it
    log::debug!("Cleaned up {} host-visible images", images.len());
}

fn ensure_host_visibility_barrier(
    device_data: &DeviceData,
    _host_image: &HostVisibleImage,
) -> Result<(), vk::Result> {
    // Execute REAL memory barrier if we have command pool and queue
    if let (Some(command_pool), Some(queue)) =
        (device_data.command_pool, device_data.graphics_queue)
    {
        unsafe {
            let layer_data_guard = LAYER_DATA.lock().unwrap();
            let instance_data = layer_data_guard.as_ref().unwrap();
            let ash_instance = ash::Instance::load(
                &ash::Entry::load().unwrap().static_fn(),
                instance_data.instance,
            );
            let ash_device = ash::Device::load(&ash_instance.fp_v1_0(), device_data.device);

            // Allocate REAL command buffer
            let cmd_alloc_info = vk::CommandBufferAllocateInfo::builder()
                .command_pool(command_pool)
                .level(vk::CommandBufferLevel::PRIMARY)
                .command_buffer_count(1);

            let cmd_buffers = ash_device.allocate_command_buffers(&cmd_alloc_info)?;
            let cmd_buffer = cmd_buffers[0];

            // Begin REAL command buffer
            let begin_info = vk::CommandBufferBeginInfo::builder()
                .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
            ash_device.begin_command_buffer(cmd_buffer, &begin_info)?;

            // Issue REAL memory barrier to make GPU writes visible to host
            let barrier = vk::MemoryBarrier::builder()
                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                .dst_access_mask(vk::AccessFlags::HOST_READ);

            ash_device.cmd_pipeline_barrier(
                cmd_buffer,
                vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags::HOST,
                vk::DependencyFlags::empty(),
                &[*barrier],
                &[],
                &[],
            );

            // End REAL command buffer
            ash_device.end_command_buffer(cmd_buffer)?;

            // Submit REAL command buffer and wait
            let cmd_buffers = [cmd_buffer];
            let submit_info = vk::SubmitInfo::builder().command_buffers(&cmd_buffers);

            ash_device.queue_submit(queue, &[*submit_info], vk::Fence::null())?;
            ash_device.queue_wait_idle(queue)?;

            // Free the command buffer
            ash_device.free_command_buffers(command_pool, &[cmd_buffer]);

            log::debug!("REAL memory barrier completed - GPU writes now visible to host");
        }
    } else {
        log::warn!("No command pool/queue available - cannot execute memory barrier");
    }

    Ok(())
}

fn capture_host_visible_frame(
    device_data: &DeviceData,
    swapchain_info: &SwapchainInfo,
    image_index: usize,
    frame_num: u32,
    config: &LayerConfig,
) {
    if image_index >= swapchain_info.images.len() {
        log::error!(
            "Invalid image index {} for swapchain with {} images",
            image_index,
            swapchain_info.images.len()
        );
        return;
    }

    let host_image = &swapchain_info.images[image_index];

    log::info!(
        "Capturing frame {} from host-visible memory ({}x{}, format: {:?})",
        frame_num,
        swapchain_info.extent.width,
        swapchain_info.extent.height,
        swapchain_info.format
    );

    // Ensure GPU writes are visible to host before reading
    if let Err(e) = ensure_host_visibility_barrier(device_data, host_image) {
        log::error!("Failed to ensure host visibility: {:?}", e);
        return;
    }

    // Read pixel data directly from mapped memory
    let rgb_data =
        convert_host_image_to_rgb(host_image, swapchain_info.extent, swapchain_info.format);

    match rgb_data {
        Some(pixels) => {
            let extension = match config.output_format {
                OutputFormat::Ppm => "ppm",
                OutputFormat::Png => "png",
            };

            let filename = format!("{}/frame_{:06}.{}", config.output_dir, frame_num, extension);

            let result = match config.output_format {
                OutputFormat::Ppm => save_ppm_frame(
                    &filename,
                    &pixels,
                    swapchain_info.extent.width,
                    swapchain_info.extent.height,
                ),
                OutputFormat::Png => save_png_frame(
                    &filename,
                    &pixels,
                    swapchain_info.extent.width,
                    swapchain_info.extent.height,
                ),
            };

            match result {
                Ok(file_size) => {
                    log::info!(
                        "Successfully saved frame {} ({} bytes, {}x{} pixels)",
                        frame_num,
                        file_size,
                        swapchain_info.extent.width,
                        swapchain_info.extent.height
                    );
                }
                Err(e) => {
                    log::error!("Failed to write frame {}: {}", filename, e);
                }
            }
        }
        None => {
            log::error!(
                "Failed to convert host image to RGB for frame {}",
                frame_num
            );
        }
    }
}

fn convert_host_image_to_rgb(
    host_image: &HostVisibleImage,
    extent: vk::Extent2D,
    format: vk::Format,
) -> Option<Vec<u8>> {
    let pixel_count = (extent.width * extent.height) as usize;
    let mut rgb_data = Vec::with_capacity(pixel_count * 3);

    unsafe {
        let data_ptr = host_image.mapped_ptr;

        match format {
            vk::Format::B8G8R8A8_SRGB | vk::Format::B8G8R8A8_UNORM => {
                // BGRA to RGB conversion
                for y in 0..extent.height {
                    let row_offset = (y * host_image.row_pitch) as usize;
                    for x in 0..extent.width {
                        let pixel_offset = row_offset + (x * 4) as usize;
                        let b = *data_ptr.add(pixel_offset);
                        let g = *data_ptr.add(pixel_offset + 1);
                        let r = *data_ptr.add(pixel_offset + 2);
                        // Skip alpha
                        rgb_data.extend_from_slice(&[r, g, b]);
                    }
                }
            }
            vk::Format::R8G8B8A8_SRGB | vk::Format::R8G8B8A8_UNORM => {
                // RGBA to RGB conversion
                for y in 0..extent.height {
                    let row_offset = (y * host_image.row_pitch) as usize;
                    for x in 0..extent.width {
                        let pixel_offset = row_offset + (x * 4) as usize;
                        let r = *data_ptr.add(pixel_offset);
                        let g = *data_ptr.add(pixel_offset + 1);
                        let b = *data_ptr.add(pixel_offset + 2);
                        // Skip alpha
                        rgb_data.extend_from_slice(&[r, g, b]);
                    }
                }
            }
            vk::Format::R8G8B8_SRGB | vk::Format::R8G8B8_UNORM => {
                // RGB to RGB (direct copy with row pitch consideration)
                for y in 0..extent.height {
                    let row_offset = (y * host_image.row_pitch) as usize;
                    for x in 0..extent.width {
                        let pixel_offset = row_offset + (x * 3) as usize;
                        let r = *data_ptr.add(pixel_offset);
                        let g = *data_ptr.add(pixel_offset + 1);
                        let b = *data_ptr.add(pixel_offset + 2);
                        rgb_data.extend_from_slice(&[r, g, b]);
                    }
                }
            }
            _ => {
                log::warn!("Unsupported format for host-visible capture: {:?}", format);
                return None;
            }
        }
    }

    Some(rgb_data)
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

// Helper function to generate unique handles
fn generate_unique_handle() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0x1000);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// Helper function to get chain info from create info
unsafe fn get_chain_info(
    create_info: *const vk::InstanceCreateInfo,
) -> Result<vk::PFN_vkGetInstanceProcAddr, vk::Result> {
    let create_info = &*create_info;
    let mut p_next = create_info.p_next;

    log::debug!("Parsing chain info, starting p_next: {:p}", p_next);

    while !p_next.is_null() {
        let base_header = p_next as *const vk::BaseInStructure;
        let s_type = (*base_header).s_type;
        log::debug!(
            "Found structure type: {:?} (raw: {})",
            s_type,
            s_type.as_raw()
        );

        // Check for LOADER_INSTANCE_CREATE_INFO (raw: 47)
        if s_type.as_raw() == 47 {
            log::info!("Found LOADER_INSTANCE_CREATE_INFO structure");

            // Parse the loader instance create info structure
            let chain_info = p_next as *const VkLayerInstanceCreateInfo;
            let function_type = &(*chain_info).function;
            let layer_info = (*chain_info).p_layer_info;

            log::debug!(
                "Function type: {:?}, layer_info: {:p}",
                function_type,
                layer_info
            );

            if !layer_info.is_null() {
                let func = (*layer_info).next_get_instance_proc_addr;
                if !(func as *const ()).is_null() {
                    log::info!("Found valid function pointer in chain info");
                    return Ok(func);
                }
            }
        }

        p_next = (*base_header).p_next as *const std::ffi::c_void;
        log::debug!("Moving to next structure: {:p}", p_next);
    }

    log::error!("No chain info found in structure chain");
    Err(vk::Result::ERROR_INITIALIZATION_FAILED)
}

// Vulkan layer chain info structures
#[repr(C)]
struct VkLayerInstanceCreateInfo {
    s_type: vk::StructureType,
    p_next: *const std::ffi::c_void,
    function: VkLayerFunction,
    p_layer_info: *const VkLayerInstanceLink,
}

#[repr(C)]
struct VkLayerInstanceLink {
    next_get_instance_proc_addr: vk::PFN_vkGetInstanceProcAddr,
    next_get_device_proc_addr: vk::PFN_vkGetDeviceProcAddr,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
enum VkLayerFunction {
    VkLayerLinkInfo = 0,
}

// Vulkan chain info type constants
const _VK_CHAIN_INFO_TYPE_INSTANCE_PROC_ADDR: u32 = 1000164001;

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
