use crate::canvas::RgbColor;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone)]
pub struct InheritedStyle {
    pub color: RgbColor,
    pub font_name: String,
    pub font_size: f32,
    pub text_align: TextAlign,
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
            text_align: TextAlign::default(),
        }
    }

    pub fn with_overrides(&self, overrides: &InheritedStyleOverrides) -> Self {
        InheritedStyle {
            color: overrides.color.unwrap_or(self.color),
            font_name: overrides
                .font_name
                .clone()
                .unwrap_or_else(|| self.font_name.clone()),
            font_size: overrides.font_size.unwrap_or(self.font_size),
            text_align: overrides.text_align.unwrap_or(self.text_align),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct InheritedStyleOverrides {
    pub color: Option<RgbColor>,
    pub font_name: Option<String>,
    pub font_size: Option<f32>,
    pub text_align: Option<TextAlign>,
}
