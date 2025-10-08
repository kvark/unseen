#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <dlfcn.h>

// Vulkan types and constants we need
typedef struct VkInstance_T* VkInstance;
typedef struct VkDevice_T* VkDevice;
typedef struct VkPhysicalDevice_T* VkPhysicalDevice;
typedef struct VkSurfaceKHR_T* VkSurfaceKHR;
typedef struct VkSwapchainKHR_T* VkSwapchainKHR;
typedef struct VkQueue_T* VkQueue;
typedef uint32_t VkBool32;
typedef uint32_t VkFlags;
typedef int VkResult;

#define VK_SUCCESS 0
#define VK_TRUE 1
#define VK_FALSE 0
#define VK_NULL_HANDLE 0

// Function pointer types
typedef VkResult (*PFN_vkCreateInstance)(void*, void*, VkInstance*);
typedef void (*PFN_vkDestroyInstance)(VkInstance, void*);
typedef VkResult (*PFN_vkCreateDevice)(VkPhysicalDevice, void*, void*, VkDevice*);
typedef void (*PFN_vkDestroyDevice)(VkDevice, void*);
typedef VkResult (*PFN_vkCreateSwapchainKHR)(VkDevice, void*, void*, VkSwapchainKHR*);
typedef void (*PFN_vkDestroySwapchainKHR)(VkDevice, VkSwapchainKHR, void*);
typedef VkResult (*PFN_vkAcquireNextImageKHR)(VkDevice, VkSwapchainKHR, uint64_t, void*, void*, uint32_t*);
typedef VkResult (*PFN_vkQueuePresentKHR)(VkQueue, void*);
typedef void* (*PFN_vkGetInstanceProcAddr)(VkInstance, const char*);
typedef void* (*PFN_vkGetDeviceProcAddr)(VkDevice, const char*);

// Simple structures
typedef struct {
    int sType;
    void* pNext;
    VkFlags flags;
    void* pApplicationInfo;
    uint32_t enabledLayerCount;
    const char* const* ppEnabledLayerNames;
    uint32_t enabledExtensionCount;
    const char* const* ppEnabledExtensionNames;
} VkInstanceCreateInfo;

typedef struct {
    int sType;
    void* pNext;
    VkFlags flags;
    uint32_t queueCreateInfoCount;
    void* pQueueCreateInfos;
    uint32_t enabledLayerCount;
    const char* const* ppEnabledLayerNames;
    uint32_t enabledExtensionCount;
    const char* const* ppEnabledExtensionNames;
    void* pEnabledFeatures;
} VkDeviceCreateInfo;

typedef struct {
    int sType;
    void* pNext;
    VkFlags flags;
    VkSurfaceKHR surface;
    uint32_t minImageCount;
    int imageFormat;
    int imageColorSpace;
    struct { uint32_t width, height; } imageExtent;
    uint32_t imageArrayLayers;
    VkFlags imageUsage;
    int imageSharingMode;
    uint32_t queueFamilyIndexCount;
    uint32_t* pQueueFamilyIndices;
    int preTransform;
    int compositeAlpha;
    int presentMode;
    VkBool32 clipped;
    VkSwapchainKHR oldSwapchain;
} VkSwapchainCreateInfoKHR;

typedef struct {
    int sType;
    void* pNext;
    uint32_t waitSemaphoreCount;
    void* pWaitSemaphores;
    uint32_t swapchainCount;
    VkSwapchainKHR* pSwapchains;
    uint32_t* pImageIndices;
    VkResult* pResults;
} VkPresentInfoKHR;

#define VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO 1
#define VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO 3
#define VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR 1000001000
#define VK_STRUCTURE_TYPE_PRESENT_INFO_KHR 1000001001

int main() {
    printf("=== Simple Headless Layer Test ===\n");
    printf("Testing Vulkan layer functionality with direct function calls\n\n");

    // Load the layer library directly
    void* layer_lib = dlopen("./target/release/libVkLayer_PRIVATE_unseen.so", RTLD_LAZY);
    if (!layer_lib) {
        printf("❌ Failed to load layer library: %s\n", dlerror());
        return 1;
    }
    printf("✅ Layer library loaded successfully\n");

    // Get function pointers
    PFN_vkGetInstanceProcAddr vkGetInstanceProcAddr = 
        (PFN_vkGetInstanceProcAddr)dlsym(layer_lib, "vkGetInstanceProcAddr");
    
    if (!vkGetInstanceProcAddr) {
        printf("❌ Failed to get vkGetInstanceProcAddr\n");
        dlclose(layer_lib);
        return 1;
    }
    printf("✅ Got vkGetInstanceProcAddr\n");

    // Get instance functions
    PFN_vkCreateInstance vkCreateInstance = 
        (PFN_vkCreateInstance)vkGetInstanceProcAddr(VK_NULL_HANDLE, "vkCreateInstance");
    PFN_vkDestroyInstance vkDestroyInstance = 
        (PFN_vkDestroyInstance)vkGetInstanceProcAddr(VK_NULL_HANDLE, "vkDestroyInstance");
    PFN_vkCreateDevice vkCreateDevice = 
        (PFN_vkCreateDevice)vkGetInstanceProcAddr(VK_NULL_HANDLE, "vkCreateDevice");

    if (!vkCreateInstance || !vkDestroyInstance || !vkCreateDevice) {
        printf("❌ Failed to get instance functions\n");
        dlclose(layer_lib);
        return 1;
    }
    printf("✅ Got instance functions\n");

    // Create instance
    VkInstanceCreateInfo instance_info = {0};
    instance_info.sType = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO;
    
    VkInstance instance = VK_NULL_HANDLE;
    VkResult result = vkCreateInstance(&instance_info, NULL, &instance);
    if (result != VK_SUCCESS) {
        printf("❌ Failed to create instance: %d\n", result);
        dlclose(layer_lib);
        return 1;
    }
    printf("✅ Instance created: %p\n", (void*)instance);

    // Create device (using dummy physical device)
    VkDeviceCreateInfo device_info = {0};
    device_info.sType = VK_STRUCTURE_TYPE_DEVICE_CREATE_INFO;
    
    VkDevice device = VK_NULL_HANDLE;
    result = vkCreateDevice((VkPhysicalDevice)0x1234, &device_info, NULL, &device);
    if (result != VK_SUCCESS) {
        printf("❌ Failed to create device: %d\n", result);
        vkDestroyInstance(instance, NULL);
        dlclose(layer_lib);
        return 1;
    }
    printf("✅ Device created: %p\n", (void*)device);

    // Get device functions
    PFN_vkGetDeviceProcAddr vkGetDeviceProcAddr = 
        (PFN_vkGetDeviceProcAddr)vkGetInstanceProcAddr(instance, "vkGetDeviceProcAddr");
    PFN_vkCreateSwapchainKHR vkCreateSwapchainKHR = 
        (PFN_vkCreateSwapchainKHR)vkGetDeviceProcAddr(device, "vkCreateSwapchainKHR");
    PFN_vkDestroySwapchainKHR vkDestroySwapchainKHR = 
        (PFN_vkDestroySwapchainKHR)vkGetDeviceProcAddr(device, "vkDestroySwapchainKHR");
    PFN_vkAcquireNextImageKHR vkAcquireNextImageKHR = 
        (PFN_vkAcquireNextImageKHR)vkGetDeviceProcAddr(device, "vkAcquireNextImageKHR");
    PFN_vkQueuePresentKHR vkQueuePresentKHR = 
        (PFN_vkQueuePresentKHR)vkGetDeviceProcAddr(device, "vkQueuePresentKHR");

    if (!vkGetDeviceProcAddr || !vkCreateSwapchainKHR || !vkAcquireNextImageKHR || !vkQueuePresentKHR) {
        printf("❌ Failed to get device functions\n");
        vkDestroyInstance(instance, NULL);
        dlclose(layer_lib);
        return 1;
    }
    printf("✅ Got device functions\n");

    // Create swapchain
    VkSwapchainCreateInfoKHR swapchain_info = {0};
    swapchain_info.sType = VK_STRUCTURE_TYPE_SWAPCHAIN_CREATE_INFO_KHR;
    swapchain_info.surface = (VkSurfaceKHR)0x5678;  // Dummy surface
    swapchain_info.minImageCount = 3;
    swapchain_info.imageFormat = 37; // VK_FORMAT_B8G8R8A8_SRGB
    swapchain_info.imageColorSpace = 0; // VK_COLOR_SPACE_SRGB_NONLINEAR_KHR
    swapchain_info.imageExtent.width = 1920;
    swapchain_info.imageExtent.height = 1080;
    swapchain_info.imageArrayLayers = 1;
    swapchain_info.imageUsage = 0x10; // VK_IMAGE_USAGE_COLOR_ATTACHMENT_BIT
    swapchain_info.imageSharingMode = 0; // VK_SHARING_MODE_EXCLUSIVE
    swapchain_info.preTransform = 1; // VK_SURFACE_TRANSFORM_IDENTITY_BIT_KHR
    swapchain_info.compositeAlpha = 1; // VK_COMPOSITE_ALPHA_OPAQUE_BIT_KHR
    swapchain_info.presentMode = 2; // VK_PRESENT_MODE_FIFO_KHR
    swapchain_info.clipped = VK_TRUE;
    swapchain_info.oldSwapchain = VK_NULL_HANDLE;

    VkSwapchainKHR swapchain = VK_NULL_HANDLE;
    result = vkCreateSwapchainKHR(device, &swapchain_info, NULL, &swapchain);
    if (result != VK_SUCCESS) {
        printf("❌ Failed to create swapchain: %d\n", result);
        vkDestroyInstance(instance, NULL);
        dlclose(layer_lib);
        return 1;
    }
    printf("✅ Swapchain created: %p (1920x1080)\n", (void*)swapchain);

    // Simulate rendering loop
    printf("\n🎬 Simulating frame rendering and capture...\n");
    for (int frame = 0; frame < 10; frame++) {
        // Acquire next image
        uint32_t image_index = 0;
        result = vkAcquireNextImageKHR(device, swapchain, UINT64_MAX, NULL, NULL, &image_index);
        if (result != VK_SUCCESS) {
            printf("❌ Failed to acquire image for frame %d: %d\n", frame, result);
            break;
        }

        printf("   📸 Frame %02d: acquired image %u", frame, image_index);

        // Present the frame (this triggers capture in our layer)
        VkPresentInfoKHR present_info = {0};
        present_info.sType = VK_STRUCTURE_TYPE_PRESENT_INFO_KHR;
        present_info.swapchainCount = 1;
        present_info.pSwapchains = &swapchain;
        present_info.pImageIndices = &image_index;

        result = vkQueuePresentKHR((VkQueue)0x9ABC, &present_info);
        if (result == VK_SUCCESS) {
            printf(" → ✅ presented\n");
        } else {
            printf(" → ❌ present failed: %d\n", result);
            break;
        }
    }

    // Cleanup
    printf("\n🧹 Cleaning up...\n");
    if (vkDestroySwapchainKHR) {
        vkDestroySwapchainKHR(device, swapchain, NULL);
        printf("✅ Swapchain destroyed\n");
    }

    PFN_vkDestroyDevice vkDestroyDevice = 
        (PFN_vkDestroyDevice)vkGetDeviceProcAddr(device, "vkDestroyDevice");
    if (vkDestroyDevice) {
        vkDestroyDevice(device, NULL);
        printf("✅ Device destroyed\n");
    }

    vkDestroyInstance(instance, NULL);
    printf("✅ Instance destroyed\n");

    dlclose(layer_lib);
    printf("✅ Layer library unloaded\n");

    printf("\n🎉 Simple headless test completed!\n");
    printf("📁 Check the captured_frames directory for output files\n");

    return 0;
}