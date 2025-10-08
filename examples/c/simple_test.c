#define _POSIX_C_SOURCE 200112L
#include <vulkan/vulkan.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

int main() {
    printf("🎬 Simple Vulkan Test with Unseen Layer\n");
    printf("========================================\n\n");

    // Create application info
    VkApplicationInfo app_info;
    memset(&app_info, 0, sizeof(app_info));
    app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    app_info.pApplicationName = "Simple Unseen Test";
    app_info.applicationVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.pEngineName = "Test Engine";
    app_info.engineVersion = VK_MAKE_VERSION(1, 0, 0);
    app_info.apiVersion = VK_API_VERSION_1_0;

    // Create instance
    VkInstanceCreateInfo instance_info;
    memset(&instance_info, 0, sizeof(instance_info));
    instance_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    instance_info.pApplicationInfo = &app_info;

    VkInstance instance;
    VkResult result = vkCreateInstance(&instance_info, NULL, &instance);
    if (result != VK_SUCCESS) {
        printf("❌ Failed to create instance: %d\n", result);
        return 1;
    }
    printf("✅ Vulkan instance created\n");

    // Create device
    VkDeviceCreateInfo device_info;
    memset(&device_info, 0, sizeof(device_info));
    device_info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;

    VkDevice device;
    result = vkCreateDevice(VK_NULL_HANDLE, &device_info, NULL, &device);
    if (result != VK_SUCCESS) {
        printf("❌ Failed to create device: %d\n", result);
        return 1;
    }
    printf("✅ Vulkan device created\n");

    // Create swapchain
    VkSwapchainCreateInfoKHR swapchain_info;
    memset(&swapchain_info, 0, sizeof(swapchain_info));
    swapchain_info.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    swapchain_info.surface = VK_NULL_HANDLE;
    swapchain_info.minImageCount = 3;
    swapchain_info.imageFormat = VK_FORMAT_B8G8R8A8_UNORM;
    swapchain_info.imageColorSpace = VK_COLOR_SPACE_SRGB_NONLINEAR_KHR;
    swapchain_info.imageExtent.width = 800;
    swapchain_info.imageExtent.height = 600;
    swapchain_info.imageArrayLayers = 1;
    swapchain_info.imageUsage = VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT;
    swapchain_info.imageSharingMode = VK_SHARING_MODE_EXCLUSIVE;
    swapchain_info.preTransform = VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR;
    swapchain_info.compositeAlpha = VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR;
    swapchain_info.presentMode = VK_PRESENT_MODE_FIFO_KHR;
    swapchain_info.clipped = VK_TRUE;

    VkSwapchainKHR swapchain;
    result = vkCreateSwapchainKHR(device, &swapchain_info, NULL, &swapchain);
    if (result != VK_SUCCESS) {
        printf("❌ Failed to create swapchain: %d\n", result);
        return 1;
    }
    printf("✅ Swapchain created (800x600)\n\n");

    printf("📸 Presenting frames for capture...\n");

    // Present frames - this should trigger our capture layer
    VkPresentInfoKHR present_info;
    memset(&present_info, 0, sizeof(present_info));
    present_info.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
    present_info.swapchainCount = 1;
    present_info.pSwapchains = &swapchain;
    uint32_t image_index = 0;
    present_info.pImageIndices = &image_index;

    for (int i = 0; i < 5; i++) {
        printf("   Frame %02d: ", i);
        fflush(stdout);

        result = vkQueuePresentKHR(VK_NULL_HANDLE, &present_info);
        if (result == VK_SUCCESS) {
            printf("✅ presented\n");
        } else {
            printf("❌ failed (%d)\n", result);
        }

        // Brief pause between frames
        sleep(1); // 1 second
    }

    printf("\n🧹 Cleaning up...\n");
    vkDestroySwapchainKHR(device, swapchain, NULL);
    vkDestroyDevice(device, NULL);
    vkDestroyInstance(instance, NULL);

    printf("✅ Test completed successfully!\n");
    printf("\n📁 Check the captured_frames directory for captured frames\n");
    
    return 0;
}