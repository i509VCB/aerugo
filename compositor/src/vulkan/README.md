Some notes regarding what needs to be done in the Vulkan abstractions:

1. Consider a better way to ensure the device builder has at least one queue.
2. Shader compilation and loading.
3. Wrappers around gpu synchronization primitives. Things like (timeline) semaphores, fences, barriers.
4. Memory allocation. Specifically regarding how to represent textures and other memory objects the device creates.
5. Render passes and such (dynamic rendering is nice and all, but supporting RenderElement in smithay properly means we need render passes).
6. Supported extensions (for init)
- `VK_EXT_physical_device_drm` (DRM node to physical device).

## `VulkanRenderer`

This is a more complex part of the vulkan code, so we need to think about some things.

1. Command buffers and representing a way to queue commands inside a `Frame`. I assume a frame would record the draw commands.
2. Importing/Exporting memory is quite easy as it appears:
- shm would be implemented using one of the copy commands (https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/chap20.html) with some in renderer support for tracking damage and only copying what has changed.
- dmabuf import would be implemented using the external memory extensions (`VK_EXT_external_memory_dma_buf`) using `vkGetMemoryFdKHR` for export and the `VkImportMemoryFdInfoKHR` extension for import. The resulting imported type would become a `VkBuffer`.
3. `Frame::clear` can be implemented using `vkCmdClearAttachments`. This supports damage using the `pRects` parameter.
4. Rendering to a dmabuf involves using the dmabuf (image when imported) as a framebuffer.
5. Rendering a texture appears to be possible using several methods using some of the copy commands (https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/chap20.html).
- Some of these commands have varying targets, supported formats and whether scaling is performed. Pretty much all of these commands allow specifying the regions to copy.
6. Required extensions
- `VK_EXT_image_drm_format_modifier`
- `VK_EXT_external_memory_dma_buf` and `VK_KHR_external_memory_fd`
