# Sorenon OpenXR Layer
(I'm not good with names)


As of writing, both the SteamVR and WMR OpenXR runtimes have a few significant issues / missing features. 
WMR does not provide any OpenGL / Vulkan extensions and SteamVR on Linux is essentially broken.

WMR's issues can be fixed through the use of SteamVR. <br>
[This existing layer](https://github.com/ChristophHaag/gl_context_fix_layer) also attempts to fix SteamVR's GLX support and may be more performant (but has some stability issues).


## How the layer works
When the application creates an OpenGL session, the layer creates a Vulkan session and uses external memory extensions to share swapchain images between the apis. 
This adds the extra overhead of creating a second swapchain to expose to the application, and one draw call in `xrReleaseSwapchainImage` to copy and transfrom the image into the OpenXR swapchain.
<br><br>
There are some notable areas that can be improved. Mainly using an interop semephore instead of a `glFinish` call and passing a fence to an async thread instead of a `vkQueueWaitIdle` call.

## Fixes:
- https://github.com/ValveSoftware/SteamVR-for-Linux/issues/421
- https://github.com/ValveSoftware/SteamVR-for-Linux/issues/466

## Does not fix:
- https://github.com/ValveSoftware/SteamVR-for-Linux/issues/422<br>^ Is it possible to fix this by calling xrDestroyInstance in a new thread?
- https://github.com/ValveSoftware/SteamVR-for-Linux/issues/461
- https://github.com/ValveSoftware/SteamVR-for-Linux/issues/479

## Current TODO:
- [x] OpenGL Frontend
- [x] Vulkan Backend
- [ ] Linux Installer
- [ ] Correctly handle sRGB formats
- [ ] Investigate improving performance

## If perfomance impact can be minimized:
- [ ] D3D11 Backend
- [ ] Vulkan Frontend
- [ ] Windows Installer
- [ ] FSR / NIS
- [ ] Attempt to deal with other runtime bugs
