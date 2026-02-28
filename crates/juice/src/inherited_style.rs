use crate::canvas::RgbColor;

#[derive(Clone)]
pub struct InheritedStyle {
    pub color: RgbColor,
    pub font_name: String,
    pub font_size: f32,
}

impl InheritedStyle {
    pub fn new(default_font: &str) -> Self {
        InheritedStyle {
            color: RgbColor {
                r: 255,
                g: 255,
                b: 255,
            },
            font_name: default_font.to_string(),
            font_size: 24.0,
        }
    }

    pub fn clone_and_override(
        &self,
        color: Option<RgbColor>,
        font_name: Option<String>,
        font_size: Option<f32>,
    ) -> Self {
        let mut cloned = self.clone();

        if let Some(color) = color {
            cloned.color = color;
        }

        if let Some(font_name) = font_name {
            cloned.font_name = font_name.clone();
        }

        if let Some(font_size) = font_size {
            cloned.font_size = font_size;
        }

        cloned
    }
}
