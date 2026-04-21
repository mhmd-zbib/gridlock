use glam::Vec2;

#[derive(Debug, Clone, Copy)]
pub struct PrimaryButtonStyle {
    pub fill: [f32; 4],
    pub fill_hover: [f32; 4],
    pub stroke: [f32; 4],
    pub slash_glow: [f32; 4],
    pub text: [f32; 4],
    pub corner_cut_px: f32,
    pub border_px: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct PrimaryButtonFrame {
    pub center: Vec2,
    pub size: Vec2,
    pub skew_px: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct PrimaryButtonQuad {
    pub center: [f32; 2],
    pub half_size: [f32; 2],
    pub top_color: [f32; 4],
    pub bottom_color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub accent_color: [f32; 4],
    pub params: [f32; 4],
}

pub struct PrimaryButton;

impl PrimaryButtonQuad {
    fn from_center_size(
        center: Vec2,
        size: Vec2,
        top_color: [f32; 4],
        bottom_color: [f32; 4],
        stroke_color: [f32; 4],
        accent_color: [f32; 4],
        params: [f32; 4],
    ) -> Self {
        Self {
            center: [center.x, center.y],
            half_size: [size.x * 0.5, size.y * 0.5],
            top_color,
            bottom_color,
            stroke_color,
            accent_color,
            params,
        }
    }
}

impl PrimaryButtonFrame {
    pub fn centered(center: [f32; 2], size: [f32; 2], skew_px: f32) -> Self {
        Self {
            center: Vec2::new(center[0], center[1]),
            size: Vec2::new(size[0], size[1]),
            skew_px,
        }
    }
}

impl PrimaryButton {
    pub fn style() -> PrimaryButtonStyle {
        PrimaryButtonStyle {
            fill: [0.15, 0.18, 0.21, 1.0],
            fill_hover: [0.20, 0.24, 0.28, 1.0],
            stroke: [0.46, 0.53, 0.61, 0.92],
            slash_glow: [0.29, 0.72, 0.96, 0.28],
            text: [0.95, 0.97, 1.0, 1.0],
            corner_cut_px: 10.0,
            border_px: 1.5,
        }
    }

    /// Returns one shader-driven shaded button instance.
    ///
    /// The engine's shaded-quad shader handles gradient, border, accent, and corner cuts.
    pub fn shaded_instance(frame: PrimaryButtonFrame, hovered: bool) -> PrimaryButtonQuad {
        let style = Self::style();
        let fill = if hovered {
            style.fill_hover
        } else {
            style.fill
        };
        let stroke = if hovered {
            mix_rgba(style.stroke, [1.0, 1.0, 1.0, style.stroke[3]], 0.12)
        } else {
            style.stroke
        };

        let top_color = mix_rgba(
            fill,
            [1.0, 1.0, 1.0, fill[3]],
            if hovered { 0.22 } else { 0.16 },
        );
        let bottom_color = mix_rgba(
            fill,
            [0.0, 0.0, 0.0, fill[3]],
            if hovered { 0.42 } else { 0.34 },
        );

        let accent_color = {
            let mut c = style.slash_glow;
            c[3] = (c[3] + if hovered { 0.18 } else { 0.08 }).min(1.0);
            c
        };

        let h = frame.size.y.max(1.0);
        let w = frame.size.x.max(1.0);
        let border_frac = (style.border_px / h).clamp(0.01, 0.22);
        let corner_cut_frac = (style.corner_cut_px / h).clamp(0.0, 0.40);
        let accent_width_frac = (0.14 + (frame.skew_px / w) * 0.10).clamp(0.08, 0.30);
        let hover_strength = if hovered { 1.0 } else { 0.0 };

        PrimaryButtonQuad::from_center_size(
            frame.center,
            frame.size,
            top_color,
            bottom_color,
            stroke,
            accent_color,
            [
                border_frac,
                corner_cut_frac,
                accent_width_frac,
                hover_strength,
            ],
        )
    }
}

fn mix_rgba(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
        a[3] + (b[3] - a[3]) * t,
    ]
}
