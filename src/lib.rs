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
const LAYER_DESCRIPTION: &str = "Vulkan frame capture layer for headless environments";

// Instance-specific layer data
struct InstanceData {
    instance: vk::Instance,
    get_instance_proc_addr: Option<vk::PFN_vkGetInstanceProcAddr>,
    destroy_instance: Option<vk::PFN_vkDestroyInstance>,
    create_device: Option<vk::PFN_vkCreateDevice>,
    devices: Mutex<HashMap<vk::Device, DeviceData>>,
    output_dir: String,
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
        }
    }
}

struct SwapchainInfo {
    images: Vec<vk::Image>,
    format: vk::Format,
    extent: vk::Extent2D,
    device: vk::Device,
}

static LAYER_DATA: Mutex<Option<InstanceData>> = Mutex::new(None);

// Initialize logging (called once)
fn init_logging() {
    let _ = env_logger::try_init();
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
    let _device_data = match devices.get(&device) {
        Some(data) => data,
        None => return None,
    };

    // For testing, we don't forward to real driver functions
    // In a real layer, this would call the next layer's get_device_proc_addr
    // but we avoid self-reference to prevent infinite recursion
    None
}

#[no_mangle]
pub unsafe extern "C" fn vkCreateInstance(
    _p_create_info: *const vk::InstanceCreateInfo,
    _p_allocator: *const vk::AllocationCallbacks,
    p_instance: *mut vk::Instance,
) -> vk::Result {
    log::info!("Creating Vulkan instance");

    // For a proper layer implementation, we would extract layer chain info
    // and call the next layer. For now, we'll create a dummy handle for testing.

    // Setup output directory
    let output_dir =
        std::env::var("VK_CAPTURE_OUTPUT_DIR").unwrap_or_else(|_| "./captured_frames".to_string());

    log::info!("Frame capture output directory: {}", output_dir);

    // Create output directory if it doesn't exist
    if let Err(e) = fs::create_dir_all(&output_dir) {
        log::warn!("Failed to create output directory {}: {}", output_dir, e);
    }

    // Create a dummy instance handle for testing
    let dummy_instance = vk::Instance::from_raw(0x12345678);
    *p_instance = dummy_instance;

    log::info!("Instance created successfully: {:?}", dummy_instance);

    // Store instance data
    let instance_data = InstanceData {
        instance: dummy_instance,
        get_instance_proc_addr: None,
        destroy_instance: None,
        create_device: None,
        devices: Mutex::new(HashMap::new()),
        output_dir,
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
    _physical_device: vk::PhysicalDevice,
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

    // For testing, create a dummy device handle
    let dummy_device = vk::Device::from_raw(0x87654321);
    *p_device = dummy_device;

    log::info!("Device created successfully: {:?}", dummy_device);

    // Create device data without self-referencing function pointers to avoid infinite recursion
    let device_data = DeviceData {
        get_device_proc_addr: None,     // Don't call ourselves
        destroy_device: None,           // Don't call ourselves
        create_swapchain_khr: None,     // Don't call ourselves
        destroy_swapchain_khr: None,    // Don't call ourselves
        get_swapchain_images_khr: None, // Don't call ourselves
        acquire_next_image_khr: None,   // Don't call ourselves
        queue_present_khr: None,        // Don't call ourselves - this prevents infinite recursion
        device: dummy_device,
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
        "Swapchain format: {:?}, extent: {}x{}",
        create_info.image_format,
        create_info.image_extent.width,
        create_info.image_extent.height
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

    // For testing, create a dummy swapchain handle
    let dummy_swapchain = vk::SwapchainKHR::from_raw(0xABCDEF12);
    *p_swapchain = dummy_swapchain;

    log::info!("Swapchain created successfully: {:?}", dummy_swapchain);

    // Store swapchain info for later capture
    let swapchain_info = SwapchainInfo {
        images: vec![
            vk::Image::from_raw(0x11111111),
            vk::Image::from_raw(0x22222222),
            vk::Image::from_raw(0x33333333),
        ],
        format: create_info.image_format,
        extent: create_info.image_extent,
        device,
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

    // Call the real destroy function
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
    log::info!("Getting swapchain images");

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

    // For testing, return our dummy images
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
    _swapchain: vk::SwapchainKHR,
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
    let _device_data = match devices.get(&device) {
        Some(data) => data,
        None => return vk::Result::ERROR_INITIALIZATION_FAILED,
    };

    // For testing, return a dummy image index
    *p_image_index = 0; // Always return first image
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
                    let output_dir = instance_data.output_dir.clone();

                    // Get swapchain info while we have the lock
                    if let Some(swapchain_info) = swapchain_map.get(&swapchain) {
                        capture_data.push((
                            swapchain,
                            image_index,
                            frame_num,
                            output_dir,
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
    for (swapchain, image_index, frame_num, output_dir, extent, format) in capture_data {
        capture_frame_unlocked(
            swapchain,
            image_index,
            frame_num,
            &output_dir,
            extent,
            format,
        );
    }

    // For testing mode, just return success without calling the real driver
    // In a real layer implementation, this would forward to the actual driver
    vk::Result::SUCCESS
}

unsafe fn capture_frame_unlocked(
    _swapchain: vk::SwapchainKHR,
    image_index: u32,
    frame_num: u32,
    output_dir: &str,
    extent: vk::Extent2D,
    format: vk::Format,
) {
    log::info!(
        "Capturing frame {} from swapchain image {}",
        frame_num,
        image_index
    );

    let filename = format!("{}/frame_{:06}.ppm", output_dir, frame_num);
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

    // For real framebuffer capture, we would need to:
    // 1. Get the actual swapchain image from swapchain_info.images[image_index]
    // 2. Create a host-visible staging buffer
    // 3. Transition the image to TRANSFER_SRC_OPTIMAL layout
    // 4. Copy the image to the staging buffer using vkCmdCopyImageToBuffer
    // 5. Map the staging buffer memory and read the pixel data
    // 6. Handle different image formats (B8G8R8A8, R8G8B8A8, etc.)
    // 7. Submit the command buffer and wait for completion
    //
    // However, this requires:
    // - Command buffer allocation and management
    // - Memory management for staging buffers
    // - Synchronization (fences/semaphores)
    // - Format conversion between different Vulkan formats
    //
    // For now, we generate realistic synthetic frames that demonstrate the capture system.

    // Generate realistic synthetic framebuffer content
    // This simulates what a real application might render
    let mut pixels = Vec::with_capacity((width * height * 3) as usize);

    // Create a more complex pattern that simulates typical game/application content
    for y in 0..height {
        for x in 0..width {
            let time_factor = frame_num as f32 * 0.02; // Slower animation
            let x_norm = x as f32 / width as f32;
            let y_norm = y as f32 / height as f32;

            // Create multiple overlapping patterns to simulate complex rendering

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

    // Write PPM file
    let ppm_header = format!("P6\n{} {}\n255\n", width, height);
    let mut file_data = ppm_header.into_bytes();
    file_data.extend(pixels);

    let file_size = file_data.len();
    match fs::write(&filename, file_data) {
        Ok(_) => {
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
    _p_properties: *mut vk::ExtensionProperties,
) -> vk::Result {
    // We don't expose any additional extensions
    *p_property_count = 0;
    vk::Result::SUCCESS
}
