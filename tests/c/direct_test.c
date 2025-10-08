#include <stdio.h>
#include <stdlib.h>
#include <dlfcn.h>
#include <vulkan/vulkan.h>
#include <string.h>

// Direct test of the capture layer by loading it as a shared library
// This bypasses the Vulkan loader complexity and directly calls our functions

int main() {
    printf("🧪 Direct Vulkan Layer Test\n");
    printf("===========================\n\n");

    // Load our layer library directly
    void* layer_lib = dlopen("./libVkLayer_PRIVATE_unseen.so", RTLD_LAZY);
    if (!layer_lib) {
        printf("❌ Failed to load layer library: %s\n", dlerror());
        return 1;
    }
    printf("✅ Layer library loaded successfully\n");

    // Get function pointers from our layer
    typedef VkResult (*CreateInstanceFunc)(const VkInstanceCreateInfo*, const VkAllocationCallbacks*, VkInstance*);
    typedef VkResult (*CreateDeviceFunc)(VkPhysicalDevice, const VkDeviceCreateInfo*, const VkAllocationCallbacks*, VkDevice*);
    typedef VkResult (*CreateSwapchainFunc)(VkDevice, const VkSwapchainCreateInfoKHR*, const VkAllocationCallbacks*, VkSwapchainKHR*);
    typedef VkResult (*QueuePresentFunc)(VkQueue, const VkPresentInfoKHR*);
    typedef void (*DestroySwapchainFunc)(VkDevice, VkSwapchainKHR, const VkAllocationCallbacks*);
    typedef void (*DestroyDeviceFunc)(VkDevice, const VkAllocationCallbacks*);
    typedef void (*DestroyInstanceFunc)(VkInstance, const VkAllocationCallbacks*);

    CreateInstanceFunc vkCreateInstance_layer = (CreateInstanceFunc)dlsym(layer_lib, "vkCreateInstance");
    CreateDeviceFunc vkCreateDevice_layer = (CreateDeviceFunc)dlsym(layer_lib, "vkCreateDevice");
    CreateSwapchainFunc vkCreateSwapchainKHR_layer = (CreateSwapchainFunc)dlsym(layer_lib, "vkCreateSwapchainKHR");
    QueuePresentFunc vkQueuePresentKHR_layer = (QueuePresentFunc)dlsym(layer_lib, "vkQueuePresentKHR");
    DestroySwapchainFunc vkDestroySwapchainKHR_layer = (DestroySwapchainFunc)dlsym(layer_lib, "vkDestroySwapchainKHR");
    DestroyDeviceFunc vkDestroyDevice_layer = (DestroyDeviceFunc)dlsym(layer_lib, "vkDestroyDevice");
    DestroyInstanceFunc vkDestroyInstance_layer = (DestroyInstanceFunc)dlsym(layer_lib, "vkDestroyInstance");

    if (!vkCreateInstance_layer || !vkCreateDevice_layer || !vkCreateSwapchainKHR_layer || !vkQueuePresentKHR_layer) {
        printf("❌ Failed to get layer function pointers\n");
        dlclose(layer_lib);
        return 1;
    }
    printf("✅ Layer function pointers obtained\n\n");

    // Test instance creation
    printf("🔧 Testing instance creation...\n");
    VkApplicationInfo app_info;
    memset(&app_info, 0, sizeof(app_info));
    app_info.sType = VK_STRUCTURE_TYPE_APPLICATION_INFO;
    app_info.pApplicationName = "Direct Layer Test";
    app_info.apiVersion = VK_API_VERSION_1_0;

    VkInstanceCreateInfo instance_info;
    memset(&instance_info, 0, sizeof(instance_info));
    instance_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    instance_info.pApplicationInfo = &app_info;

    VkInstance instance;
    VkResult result = vkCreateInstance_layer(&instance_info, NULL, &instance);
    if (result == VK_SUCCESS) {
        printf("✅ Instance created: %p\n", (void*)instance);
    } else {
        printf("❌ Instance creation failed: %d\n", result);
        dlclose(layer_lib);
        return 1;
    }

    // Test device creation
    printf("🔧 Testing device creation...\n");
    VkDeviceCreateInfo device_info;
    memset(&device_info, 0, sizeof(device_info));
    device_info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;

    VkDevice device;
    result = vkCreateDevice_layer(VK_NULL_HANDLE, &device_info, NULL, &device);
    if (result == VK_SUCCESS) {
        printf("✅ Device created: %p\n", (void*)device);
    } else {
        printf("❌ Device creation failed: %d\n", result);
        vkDestroyInstance_layer(instance, NULL);
        dlclose(layer_lib);
        return 1;
    }

    // Test swapchain creation
    printf("🔧 Testing swapchain creation...\n");
    VkSwapchainCreateInfoKHR swapchain_info;
    memset(&swapchain_info, 0, sizeof(swapchain_info));
    swapchain_info.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    swapchain_info.imageFormat = VK_FORMAT_B8G8R8A8_UNORM;
    swapchain_info.imageExtent.width = 1024;
    swapchain_info.imageExtent.height = 768;
    swapchain_info.minImageCount = 3;

    VkSwapchainKHR swapchain;
    result = vkCreateSwapchainKHR_layer(device, &swapchain_info, NULL, &swapchain);
    if (result == VK_SUCCESS) {
        printf("✅ Swapchain created: %p (1024x768)\n", (void*)swapchain);
    } else {
        printf("❌ Swapchain creation failed: %d\n", result);
        vkDestroyDevice_layer(device, NULL);
        vkDestroyInstance_layer(instance, NULL);
        dlclose(layer_lib);
        return 1;
    }

    // Test frame presentation (this should capture frames!)
    printf("\n🎬 Testing frame capture...\n");
    VkPresentInfoKHR present_info;
    memset(&present_info, 0, sizeof(present_info));
    present_info.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
    present_info.swapchainCount = 1;
    present_info.pSwapchains = &swapchain;
    uint32_t image_index = 0;
    present_info.pImageIndices = &image_index;

    for (int frame = 0; frame < 15; frame++) {
        printf("   📸 Frame %02d: ", frame);
        fflush(stdout);
        
        result = vkQueuePresentKHR_layer(VK_NULL_HANDLE, &present_info);
        if (result == VK_SUCCESS) {
            printf("✅ captured\n");
        } else {
            printf("❌ failed (%d)\n", result);
            break;
        }
    }

    // Check captured frames
    printf("\n📁 Checking captured frames...\n");
    int status = system("ls -la captured_frames/ 2>/dev/null || echo '   No captured_frames directory found'");
    (void)status; // Suppress unused variable warning

    // Cleanup
    printf("\n🧹 Cleaning up...\n");
    vkDestroySwapchainKHR_layer(device, swapchain, NULL);
    vkDestroyDevice_layer(device, NULL);
    vkDestroyInstance_layer(instance, NULL);

    dlclose(layer_lib);
    
    printf("\n🎉 Direct layer test completed!\n");
    printf("📊 Results should be in the captured_frames directory\n");

    return 0;
}