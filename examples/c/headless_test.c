#include <vulkan/vulkan.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <assert.h>

#define CHECK_VK_RESULT(result) \
    do { \
        if ((result) != VK_SUCCESS) { \
            printf("Vulkan error at %s:%d: %d\n", __FILE__, __LINE__, (result)); \
            exit(1); \
        } \
    } while (0)

typedef struct {
    VkInstance instance;
    VkPhysicalDevice physical_device;
    VkDevice device;
    VkQueue graphics_queue;
    VkSurfaceKHR surface;
    VkSwapchainKHR swapchain;
    VkImage* swapchain_images;
    uint32_t swapchain_image_count;
    VkFormat swapchain_format;
    VkExtent2D swapchain_extent;
} VulkanContext;

void create_instance(VulkanContext* ctx) {
    printf("Creating Vulkan instance...\n");
    
    VkApplicationInfo app_info = {0};
    app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    app_info.pApplicationName = "Headless Test";
    app_info.applicationVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.pEngineName = "No Engine";
    app_info.engineVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.apiVersion = VK_API_VERSION_1_0;

    const char* instance_extensions[] = {
        VK_KHR_SURFACE_EXTENSION_NAME,
        VK_EXT_HEADLESS_SURFACE_EXTENSION_NAME
    };

    const char* validation_layers[] = {
        "VK_LAYER_PRIVATE_unseen"
    };

    VkInstanceCreateInfo create_info = {0};
    create_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    create_info.pApplicationInfo = &app_info;
    create_info.enabledExtensionCount = sizeof(instance_extensions) / sizeof(instance_extensions[0]);
    create_info.ppEnabledExtensionNames = instance_extensions;
    create_info.enabledLayerCount = sizeof(validation_layers) / sizeof(validation_layers[0]);
    create_info.ppEnabledLayerNames = validation_layers;

    VkResult result = vkCreateInstance(&create_info, NULL, &ctx->instance);
    CHECK_VK_RESULT(result);
    
    printf("Instance created successfully\n");
}

void pick_physical_device(VulkanContext* ctx) {
    printf("Selecting physical device...\n");
    
    uint32_t device_count = 0;
    vkEnumeratePhysicalDevices(ctx->instance, &device_count, NULL);
    
    if (device_count == 0) {
        printf("No physical devices found\n");
        exit(1);
    }
    
    VkPhysicalDevice* devices = malloc(sizeof(VkPhysicalDevice) * device_count);
    vkEnumeratePhysicalDevices(ctx->instance, &device_count, devices);
    
    // Just pick the first device for this test
    ctx->physical_device = devices[0];
    
    VkPhysicalDeviceProperties device_properties;
    vkGetPhysicalDeviceProperties(ctx->physical_device, &device_properties);
    printf("Selected device: %s\n", device_properties.deviceName);
    
    free(devices);
}

void create_surface(VulkanContext* ctx) {
    printf("Creating headless surface...\n");
    
    VkHeadlessSurfaceCreateInfoEXT create_info = {0};
    create_info.sType = VK_STRUCTURE_TYPE_HEADLESS_SURFACE_CREATE_INFO_EXT;
    
    // Get the function pointer for headless surface creation
    PFN_vkCreateHeadlessSurfaceEXT vkCreateHeadlessSurfaceEXT = 
        (PFN_vkCreateHeadlessSurfaceEXT)vkGetInstanceProcAddr(ctx->instance, "vkCreateHeadlessSurfaceEXT");
    
    if (!vkCreateHeadlessSurfaceEXT) {
        printf("vkCreateHeadlessSurfaceEXT not available\n");
        exit(1);
    }
    
    VkResult result = vkCreateHeadlessSurfaceEXT(ctx->instance, &create_info, NULL, &ctx->surface);
    CHECK_VK_RESULT(result);
    
    printf("Headless surface created successfully\n");
}

void create_logical_device(VulkanContext* ctx) {
    printf("Creating logical device...\n");
    
    // Find graphics queue family
    uint32_t queue_family_count = 0;
    vkGetPhysicalDeviceQueueFamilyProperties(ctx->physical_device, &queue_family_count, NULL);
    
    VkQueueFamilyProperties* queue_families = malloc(sizeof(VkQueueFamilyProperties) * queue_family_count);
    vkGetPhysicalDeviceQueueFamilyProperties(ctx->physical_device, &queue_family_count, queue_families);
    
    uint32_t graphics_family = UINT32_MAX;
    for (uint32_t i = 0; i < queue_family_count; i++) {
        if (queue_families[i].queueFlags & VK_QUEUE_GRAPHICS_BIT) {
            graphics_family = i;
            break;
        }
    }
    
    if (graphics_family == UINT32_MAX) {
        printf("No graphics queue family found\n");
        exit(1);
    }
    
    free(queue_families);
    
    float queue_priority = 1.0f;
    VkDeviceQueueCreateInfo queue_create_info = {0};
    queue_create_info.sType = VK_STRUCTURE_TYPE_DEVICE_QUEUE_CREATE_INFO;
    queue_create_info.queueFamilyIndex = graphics_family;
    queue_create_info.queueCount = 1;
    queue_create_info.pQueuePriorities = &queue_priority;
    
    const char* device_extensions[] = {
        VK_KHR_SWAPCHAIN_EXTENSION_NAME
    };
    
    VkDeviceCreateInfo create_info = {0};
    create_info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
    create_info.pQueueCreateInfos = &queue_create_info;
    create_info.queueCreateInfoCount = 1;
    create_info.enabledExtensionCount = sizeof(device_extensions) / sizeof(device_extensions[0]);
    create_info.ppEnabledExtensionNames = device_extensions;
    
    VkResult result = vkCreateDevice(ctx->physical_device, &create_info, NULL, &ctx->device);
    CHECK_VK_RESULT(result);
    
    vkGetDeviceQueue(ctx->device, graphics_family, 0, &ctx->graphics_queue);
    
    printf("Logical device created successfully\n");
}

void create_swapchain(VulkanContext* ctx) {
    printf("Creating swapchain...\n");
    
    // Query surface capabilities
    VkSurfaceCapabilitiesKHR capabilities;
    vkGetPhysicalDeviceSurfaceCapabilitiesKHR(ctx->physical_device, ctx->surface, &capabilities);
    
    // Query surface formats
    uint32_t format_count;
    vkGetPhysicalDeviceSurfaceFormatsKHR(ctx->physical_device, ctx->surface, &format_count, NULL);
    VkSurfaceFormatKHR* formats = malloc(sizeof(VkSurfaceFormatKHR) * format_count);
    vkGetPhysicalDeviceSurfaceFormatsKHR(ctx->physical_device, ctx->surface, &format_count, formats);
    
    // Query present modes
    uint32_t present_mode_count;
    vkGetPhysicalDeviceSurfacePresentModesKHR(ctx->physical_device, ctx->surface, &present_mode_count, NULL);
    VkPresentModeKHR* present_modes = malloc(sizeof(VkPresentModeKHR) * present_mode_count);
    vkGetPhysicalDeviceSurfacePresentModesKHR(ctx->physical_device, ctx->surface, &present_mode_count, present_modes);
    
    // Choose format
    VkSurfaceFormatKHR surface_format = formats[0];
    for (uint32_t i = 0; i < format_count; i++) {
        if (formats[i].format == VK_FORMAT_B8G8R8A8_SRGB && 
            formats[i].colorSpace == VK_COLOR_SPACE_SRGB_NONLINEAR_KHR) {
            surface_format = formats[i];
            break;
        }
    }
    
    // Choose present mode (prefer FIFO for headless)
    VkPresentModeKHR present_mode = VK_PRESENT_MODE_FIFO_KHR;
    
    // Choose extent
    VkExtent2D extent = capabilities.currentExtent;
    if (extent.width == UINT32_MAX) {
        extent.width = 1920;
        extent.height = 1080;
        
        if (extent.width < capabilities.minImageExtent.width) {
            extent.width = capabilities.minImageExtent.width;
        } else if (extent.width > capabilities.maxImageExtent.width) {
            extent.width = capabilities.maxImageExtent.width;
        }
        
        if (extent.height < capabilities.minImageExtent.height) {
            extent.height = capabilities.minImageExtent.height;
        } else if (extent.height > capabilities.maxImageExtent.height) {
            extent.height = capabilities.maxImageExtent.height;
        }
    }
    
    uint32_t image_count = capabilities.minImageCount + 1;
    if (capabilities.maxImageCount > 0 && image_count > capabilities.maxImageCount) {
        image_count = capabilities.maxImageCount;
    }
    
    VkSwapchainCreateInfoKHR create_info = {0};
    create_info.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    create_info.surface = ctx->surface;
    create_info.minImageCount = image_count;
    create_info.imageFormat = surface_format.format;
    create_info.imageColorSpace = surface_format.colorSpace;
    create_info.imageExtent = extent;
    create_info.imageArrayLayers = 1;
    create_info.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    create_info.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    create_info.preTransform = capabilities.currentTransform;
    create_info.compositeAlpha = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    create_info.presentMode = present_mode;
    create_info.clipped = VK_TRUE;
    create_info.oldSwapchain = VK_NULL_HANDLE;
    
    VkResult result = vkCreateSwapchainKHR(ctx->device, &create_info, NULL, &ctx->swapchain);
    CHECK_VK_RESULT(result);
    
    // Get swapchain images
    vkGetSwapchainImagesKHR(ctx->device, ctx->swapchain, &ctx->swapchain_image_count, NULL);
    ctx->swapchain_images = malloc(sizeof(VkImage) * ctx->swapchain_image_count);
    vkGetSwapchainImagesKHR(ctx->device, ctx->swapchain, &ctx->swapchain_image_count, ctx->swapchain_images);
    
    ctx->swapchain_format = surface_format.format;
    ctx->swapchain_extent = extent;
    
    printf("Swapchain created successfully (%dx%d, %u images)\n", 
           extent.width, extent.height, ctx->swapchain_image_count);
    
    free(formats);
    free(present_modes);
}

void simulate_rendering(VulkanContext* ctx, int frame_count) {
    printf("Simulating rendering for %d frames...\n", frame_count);
    
    for (int frame = 0; frame < frame_count; frame++) {
        // Acquire next image
        uint32_t image_index;
        VkResult result = vkAcquireNextImageKHR(ctx->device, ctx->swapchain, UINT64_MAX, 
                                                VK_NULL_HANDLE, VK_NULL_HANDLE, &image_index);
        CHECK_VK_RESULT(result);
        
        printf("Frame %d: Acquired image %u\n", frame, image_index);
        
        // In a real application, we would:
        // 1. Record command buffers
        // 2. Submit to graphics queue
        // 3. Wait for completion
        // For this test, we just simulate the present
        
        // Present the image
        VkPresentInfoKHR present_info = {0};
        present_info.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
        present_info.swapchainCount = 1;
        present_info.pSwapchains = &ctx->swapchain;
        present_info.pImageIndices = &image_index;
        
        result = vkQueuePresentKHR(ctx->graphics_queue, &present_info);
        CHECK_VK_RESULT(result);
        
        printf("Frame %d: Presented successfully\n", frame);
    }
    
    printf("Rendering simulation complete\n");
}

void cleanup(VulkanContext* ctx) {
    printf("Cleaning up...\n");
    
    if (ctx->swapchain_images) {
        free(ctx->swapchain_images);
    }
    
    if (ctx->swapchain) {
        vkDestroySwapchainKHR(ctx->device, ctx->swapchain, NULL);
    }
    
    if (ctx->surface) {
        vkDestroySurfaceKHR(ctx->instance, ctx->surface, NULL);
    }
    
    if (ctx->device) {
        vkDestroyDevice(ctx->device, NULL);
    }
    
    if (ctx->instance) {
        vkDestroyInstance(ctx->instance, NULL);
    }
    
    printf("Cleanup complete\n");
}

int main() {
    printf("=== Unseen Vulkan Layer Headless Test ===\n");
    printf("This test verifies that the layer can:\n");
    printf("1. Create a headless surface\n");
    printf("2. Create a swapchain\n");
    printf("3. Capture frames during presentation\n");
    printf("\n");
    
    VulkanContext ctx = {0};
    
    create_instance(&ctx);
    pick_physical_device(&ctx);
    create_surface(&ctx);
    create_logical_device(&ctx);
    create_swapchain(&ctx);
    
    // Simulate rendering 10 frames
    simulate_rendering(&ctx, 10);
    
    cleanup(&ctx);
    
    printf("\n=== Test Complete ===\n");
    printf("Check the captured_frames directory for output files\n");
    
    return 0;
}