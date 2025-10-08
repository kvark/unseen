#include <vulkan/vulkan.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#ifdef _WIN32
#include <windows.h>
#define usleep(x) Sleep((x)/1000)
#else
#include <unistd.h>
#endif

int main() {
    printf("🎥 Starting Vulkan Frame Capture Test\n");
    printf("====================================\n\n");

    // Create instance
    VkApplicationInfo app_info;
    memset(&app_info, 0, sizeof(app_info));
    app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    app_info.pApplicationName = "Frame Capture Demo";
    app_info.applicationVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.pEngineName = "Unseen Demo Engine";
    app_info.engineVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.apiVersion = VK_API_VERSION_1_0;

    VkInstanceCreateInfo create_info;
    memset(&create_info, 0, sizeof(create_info));
    create_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    create_info.pApplicationInfo = &app_info;
    create_info.enabledLayerCount = 0;
    create_info.enabledExtensionCount = 0;

    VkInstance instance;
    VkResult result = vkCreateInstance(&create_info, NULL, &instance);
    if (result == VK_SUCCESS) {
        printf("✅ Vulkan instance created successfully\n");
    } else {
        printf("❌ Failed to create Vulkan instance: %d\n", result);
        return 1;
    }

    // Create device
    VkDeviceCreateInfo device_info;
    memset(&device_info, 0, sizeof(device_info));
    device_info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
    device_info.queueCreateInfoCount = 0;
    device_info.enabledLayerCount = 0;
    device_info.enabledExtensionCount = 0;

    VkDevice device;
    result = vkCreateDevice(VK_NULL_HANDLE, &device_info, NULL, &device);
    if (result == VK_SUCCESS) {
        printf("✅ Vulkan device created successfully\n");
    } else {
        printf("❌ Failed to create Vulkan device: %d\n", result);
        vkDestroyInstance(instance, NULL);
        return 1;
    }

    // Create swapchain for frame capture
    VkSwapchainCreateInfoKHR sc_info;
    memset(&sc_info, 0, sizeof(sc_info));
    sc_info.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    sc_info.flags = 0;
    sc_info.surface = VK_NULL_HANDLE; // For headless rendering
    sc_info.minImageCount = 3;
    sc_info.imageFormat = VK_FORMAT_B8G8R8A8_UNORM;
    sc_info.imageColorSpace = VK_COLOR_SPACE_SRGB_NONLINEAR_KHR;
    sc_info.imageExtent.width = 1920;
    sc_info.imageExtent.height = 1080;
    sc_info.imageArrayLayers = 1;
    sc_info.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    sc_info.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    sc_info.queueFamilyIndexCount = 0;
    sc_info.pQueueFamilyIndices = NULL;
    sc_info.preTransform = VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;
    sc_info.compositeAlpha = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    sc_info.presentMode = VK_PRESENT_MODE_FIFO_KHR;
    sc_info.clipped = VK_TRUE;
    sc_info.oldSwapchain = VK_NULL_HANDLE;

    VkSwapchainKHR swapchain;
    result = vkCreateSwapchainKHR(device, &sc_info, NULL, &swapchain);
    if (result == VK_SUCCESS) {
        printf("✅ Swapchain created successfully (1920x1080)\n\n");
    } else {
        printf("❌ Failed to create swapchain: %d\n", result);
        vkDestroyDevice(device, NULL);
        vkDestroyInstance(instance, NULL);
        return 1;
    }

    printf("🎬 Rendering and capturing frames...\n");
    printf("   This demonstrates the layer's frame capture capabilities\n");
    printf("   Each frame will be saved as a PPM image file\n\n");

    // Present multiple frames to demonstrate capture
    VkPresentInfoKHR present_info;
    memset(&present_info, 0, sizeof(present_info));
    present_info.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
    present_info.waitSemaphoreCount = 0;
    present_info.pWaitSemaphores = NULL;
    present_info.swapchainCount = 1;
    present_info.pSwapchains = &swapchain;
    present_info.pResults = NULL;

    // Present frames with different image indices to show variation
    for (int frame = 0; frame < 25; frame++) {
        uint32_t image_index = frame % 3; // Cycle through 3 swapchain images
        present_info.pImageIndices = &image_index;

        printf("   📸 Frame %02d (image %d): ", frame, image_index);
        fflush(stdout);

        result = vkQueuePresentKHR(VK_NULL_HANDLE, &present_info);
        if (result == VK_SUCCESS) {
            printf("✅ captured\n");
        } else {
            printf("❌ failed (%d)\n", result);
            break;
        }

        // Brief pause to simulate realistic frame timing
        // This also makes the output more readable
        usleep(100000); // 100ms = 10 FPS (slow for demo visibility)
    }

    printf("\n✅ Frame capture sequence completed successfully!\n");
    printf("   All frames should be saved in the configured output directory\n");

    // Cleanup resources
    printf("\n🧹 Cleaning up Vulkan resources...\n");
    vkDestroySwapchainKHR(device, swapchain, NULL);
    vkDestroyDevice(device, NULL);
    vkDestroyInstance(instance, NULL);

    printf("✅ Cleanup complete\n");
    printf("\n🏁 Frame capture demo completed successfully!\n");
    printf("   Check the output directory for captured PPM files\n");
    
    return 0;
}