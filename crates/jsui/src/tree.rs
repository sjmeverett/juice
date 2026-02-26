use serde::de::{self, Deserializer};
use serde::Deserialize;

#[derive(Debug)]
pub enum WidgetNode {
    Text(String),
    Element {
        node_type: String,
        props: Props,
        children: Vec<WidgetNode>,
    },
}

#[derive(Debug, Default, Deserialize)]
pub struct Props {
    #[serde(default)]
    pub style: Option<StyleProps>,
    #[serde(default)]
    pub id: Option<u32>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StyleProps {
    // Visual
    pub background: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub border_radius: Option<f32>,
    pub color: Option<String>,
    pub font: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub font_size: Option<f32>,
    // Layout
    pub flex_direction: Option<String>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub flex_grow: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub flex_shrink: Option<f32>,
    pub width: Option<DimensionValue>,
    pub height: Option<DimensionValue>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub padding: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub padding_left: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub padding_right: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub padding_top: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub padding_bottom: Option<f32>,
    #[serde(default, deserialize_with = "deserialize_opt_px")]
    pub gap: Option<f32>,
}

/// Parse a value that can be a number or a string like "20px" into f32.
fn parse_px(s: &str) -> Option<f32> {
    s.strip_suffix("px")
        .unwrap_or(s)
        .parse::<f32>()
        .ok()
}

fn deserialize_opt_px<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<f32>, D::Error> {
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(n)) => Ok(Some(n.as_f64().unwrap_or(0.0) as f32)),
        Some(serde_json::Value::String(s)) => Ok(parse_px(&s)),
        _ => Ok(None),
    }
}

#[derive(Debug)]
pub enum DimensionValue {
    Length(f32),
    Percent(f32),
}

impl<'de> Deserialize<'de> for DimensionValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        match &value {
            serde_json::Value::Number(n) => {
                Ok(DimensionValue::Length(n.as_f64().unwrap_or(0.0) as f32))
            }
            serde_json::Value::String(s) => {
                if let Some(pct) = s.strip_suffix('%') {
                    let val = pct.parse::<f32>().map_err(de::Error::custom)?;
                    Ok(DimensionValue::Percent(val / 100.0))
                } else if let Some(px) = s.strip_suffix("px") {
                    let val = px.parse::<f32>().map_err(de::Error::custom)?;
                    Ok(DimensionValue::Length(val))
                } else if let Ok(val) = s.parse::<f32>() {
                    Ok(DimensionValue::Length(val))
                } else {
                    Err(de::Error::custom(format!("invalid dimension: {s}")))
                }
            }
            _ => Err(de::Error::custom("expected number or string for dimension")),
        }
    }
}

impl<'de> Deserialize<'de> for WidgetNode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = serde_json::Value::deserialize(deserializer)?;
        let obj = value
            .as_object()
            .ok_or_else(|| de::Error::custom("expected object"))?;

        let node_type = obj
            .get("type")
            .and_then(|v| v.as_str())
            .ok_or_else(|| de::Error::custom("missing type"))?;

        if node_type == "#text" {
            let text = obj
                .get("text")
                .map(|v| match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_default();
            Ok(WidgetNode::Text(text))
        } else {
            let props: Props = obj
                .get("props")
                .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
                .unwrap_or_default();
            let children: Vec<WidgetNode> = obj
                .get("children")
                .map(|v| serde_json::from_value(v.clone()).unwrap_or_default())
                .unwrap_or_default();
            Ok(WidgetNode::Element {
                node_type: node_type.to_string(),
                props,
                children,
            })
        }
    }
}

pub fn parse_tree(json: &str) -> Result<WidgetNode, serde_json::Error> {
    serde_json::from_str(json)
}
