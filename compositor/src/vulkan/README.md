Some notes regarding what needs to be done in the Vulkan abstractions:

2. Shader compilation and loading.

## Device enumeration

We do support using `VK_EXT_physical_device_drm` to get a physical device from a DRM node.

## Device construction

The builder is not perfect at the moment.

QueueFamilies and Queue creation need improvement.

## Synchronization primitives

These would probably be good to abstract over partially since many applications for the renderer would use synchronization primitives.

## Shaders

The OpenGL renderer does not provide utilities to create shaders.

Not sure whether the Vulkan renderer should.

# `VulkanRenderer`

## Required extensions:
- `VK_EXT_external_memory_dma_buf`
- `VK_KHR_external_memory_fd`

## Import dmabuf

Importing a dmabuf involves calling `vkAllocateMemory` with a `VkImportMemoryFdInfoKHR` extension.

The resulting `VkDeviceMemory` may be turned into a `VkImage` to be used a `VkFramebuffer` using `vkBindImageMemory`.

## Export dmabuf

Exporting a dmabuf may be done using `vkGetMemoryFdKHR`.

Every call will result in a new fd being created.

## Import Shm

Importing shared memory as a texture can be performed using one of the copy commands.

https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/chap20.html

## Export Shm

Also using copy commands.

https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/chap20.html

# `VulkanFrame` (or whatever we call it)

## `Frame::clear`
`Frame::clear` can be implemented using `vkCmdClearAttachments` as that attachment supports specifying the rectangles to clear (`pRects`).

## `Frame::render_texture_from_to`
Use one of the copy commands.

Regarding specifics, we have a some options to choose from:
| Command             | Notes | Formats |
|---------------------|-------|---------|
| `vkCmdCopyImage(2)` | No operations applied. | Formats of the src and dst are allowed to differ if they are compatible*. |
| `vkCmdBlitImage(2)` | Scaling, format conversion and filtering are possible. | TODO |

\* https://www.khronos.org/registry/vulkan/specs/1.2-extensions/html/chap43.html#formats-compatibility

# `VulkanTexture`

Alongside the required fields of `Texture`, we will provide these additional functions:

## Raw handles

`fn image() -> &ash::vk::VkImage`
`fn image_view() -> &ash::vk::VkImageView`

# Rendering process

1. There must be a bound framebuffer.

A dmabuf can be bound as a framebuffer since we import the dmabuf as a `VkImage` and use it's `VkImageView`.

If we are winit, we can use a swapchain image as the framebuffer.

2. Upload shm texture data

A dmabuf is already uploaded to the gpu since we can import the dmabuf and convert it to a `VkImage`.

The content of an shm buffers needs to be uploaded to the gpu for presentation.

I assume this would involve copying damaged parts to a staging buffer?

If the shm buffer is new, we need to create a `VkImage`.

Then we can copy the buffer contents (that have changed) to the `VkImage` using copy commands.
