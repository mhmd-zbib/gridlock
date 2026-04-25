pub mod fog;
pub mod frame;
pub mod geometry;
pub mod light;
pub mod quad;
pub mod screen;
pub mod sprite;
pub mod state;
pub mod text;

/// Depth-stencil state for reading a previously written stencil mask.
///
/// Used by every renderer that conditionally draws inside or outside the
/// FOV cone.  The cone writer uses a separate REPLACE-based state defined
/// inline in `MaskGeometryRenderer`.
pub(crate) fn stencil_read_state(compare: wgpu::CompareFunction) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: wgpu::TextureFormat::Depth24PlusStencil8,
        depth_write_enabled: Some(false),
        depth_compare: Some(wgpu::CompareFunction::Always),
        stencil: wgpu::StencilState {
            front: wgpu::StencilFaceState {
                compare,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            },
            back: wgpu::StencilFaceState {
                compare,
                fail_op: wgpu::StencilOperation::Keep,
                depth_fail_op: wgpu::StencilOperation::Keep,
                pass_op: wgpu::StencilOperation::Keep,
            },
            read_mask: 0xFF,
            write_mask: 0,
        },
        bias: wgpu::DepthBiasState::default(),
    }
}
