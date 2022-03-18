use std::slice;

use ash::vk;

use crate::vulkan::error::VkError;

pub const VERTEX_SHADER: &[u8] = include_bytes!("shader/vert.spv");

/// # Safety:
///
/// The code must be valid SPIR-V code.
///
/// Code size must also not be zero and the length of the bytes of code must be a multiple of 4.
///
/// For specific requirements, see <https://www.khronos.org/registry/vulkan/specs/1.3-extensions/man/html/VkShaderModuleCreateInfo.html>.
pub unsafe fn create_shader_module(device: &ash::Device, code: &[u8]) -> Result<vk::ShaderModule, VkError> {
    let words = unsafe { slice::from_raw_parts(code.as_ptr() as *const u32, code.len() / 4) };
    let vert_shader_module_create_info = vk::ShaderModuleCreateInfo::builder().code(words);

    Ok(unsafe { device.create_shader_module(&vert_shader_module_create_info, None) }?)
}
