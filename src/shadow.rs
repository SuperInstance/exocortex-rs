//! Shadow Rendering Pipeline — translates machine events into human stories.
//!
//! 6 stages: Raw Event → Filter → Classify → Compress → Color → Render

use crate::types::CortexEvent;

/// Shadow colors for TUI rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadowColor {
    Green,
    Yellow,
    Red,
    Blue,
    Magenta,
    Purple,
}

impl ShadowColor {
    pub fn as_str(&self) -> &'static str {
        match self {
            ShadowColor::Green => "green",
            ShadowColor::Yellow => "yellow",
            ShadowColor::Red => "red",
            ShadowColor::Blue => "blue",
            ShadowColor::Magenta => "magenta",
            ShadowColor::Purple => "purple",
        }
    }
}

/// Shadow rendering layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShadowLayer {
    Glyph,
    Narrative,
    Philosophical,
}

/// A shadow ready for the TUI.
#[derive(Debug, Clone)]
pub struct RenderedShadow {
    pub glyph: String,
    pub story: String,
    pub color: ShadowColor,
    pub layer: ShadowLayer,
    pub timestamp: f64,
    pub trace_id: String,
    pub source: String,
}

/// Classify the color of an event based on its type and confidence.
pub fn classify_color(event: &CortexEvent) -> ShadowColor {
    match event.event_type.as_str() {
        "anomaly" => ShadowColor::Red,
        "dream" => ShadowColor::Purple,
        "train" => ShadowColor::Yellow,
        "embed" | "query" | "recall" | "remember" | "agent_connect" => ShadowColor::Blue,
        "predict" => {
            if event.confidence >= 0.8 {
                ShadowColor::Green
            } else if event.confidence >= 0.4 {
                ShadowColor::Yellow
            } else {
                ShadowColor::Red
            }
        }
        "analyze" => ShadowColor::Magenta,
        _ => ShadowColor::Blue,
    }
}

/// Full shadow rendering pipeline: classify → compress → color → render.
pub fn render_shadow(event: &CortexEvent) -> RenderedShadow {
    let glyph = match event.event_type.as_str() {
        "embed" => format!("🧮 embed ({})", event.payload_get("dims").unwrap_or("")),
        "train" => format!(
            "🏋️ train {}",
            event.payload_get("model").unwrap_or("")
        ),
        "predict" => format!(
            "🧠 predict ({:.0}% conf)",
            event.confidence * 100.0
        ),
        "remember" => format!(
            "💾 remembered: {}",
            event.payload_get("preview").unwrap_or("")
        ),
        "recall" => format!(
            "🔍 recall: {}",
            event.payload_get("preview").unwrap_or("")
        ),
        "analyze" => format!(
            "📊 {}",
            event.payload_get("finding").unwrap_or("analysis")
        ),
        "query" => format!(
            "🔍 query: {}",
            event.payload_get("query").unwrap_or("")
        ),
        "anomaly" => format!(
            "⚠️ anomaly: {}",
            event.payload_get("detail").unwrap_or("")
        ),
        "dream" => format!(
            "💭 dreaming: {}",
            event.payload_get("activity").unwrap_or("")
        ),
        "resonance" => format!(
            "⚡ resonance: {} ↔ {}",
            event.payload_get("source_agent").unwrap_or("?"),
            event.payload_get("target_agent").unwrap_or("?")
        ),
        "agent_connect" => format!(
            "📡 {} joined",
            event.payload_get("agent_id").unwrap_or("agent")
        ),
        other => format!("• {}", other),
    };

    let color = classify_color(event);

    RenderedShadow {
        glyph,
        story: glyph.clone(),
        color,
        layer: ShadowLayer::Glyph,
        timestamp: event.timestamp,
        trace_id: event.trace_id.clone(),
        source: event.source.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_anomaly_red() {
        let event = CortexEvent::new("anomaly", "reflex");
        assert_eq!(classify_color(&event), ShadowColor::Red);
    }

    #[test]
    fn test_classify_dream_purple() {
        let event = CortexEvent::new("dream", "system");
        assert_eq!(classify_color(&event), ShadowColor::Purple);
    }

    #[test]
    fn test_classify_predict_confidence() {
        let high = CortexEvent::new("predict", "a").with_confidence(0.92);
        assert_eq!(classify_color(&high), ShadowColor::Green);

        let mid = CortexEvent::new("predict", "a").with_confidence(0.5);
        assert_eq!(classify_color(&mid), ShadowColor::Yellow);

        let low = CortexEvent::new("predict", "a").with_confidence(0.2);
        assert_eq!(classify_color(&low), ShadowColor::Red);
    }

    #[test]
    fn test_render_embed() {
        let event = CortexEvent::new("embed", "test")
            .with_payload("dims", "384");
        let shadow = render_shadow(&event);
        assert!(shadow.glyph.contains("🧮"));
        assert_eq!(shadow.color, ShadowColor::Blue);
    }

    #[test]
    fn test_render_anomaly() {
        let event = CortexEvent::new("anomaly", "reflex")
            .with_payload("detail", "temp spike");
        let shadow = render_shadow(&event);
        assert!(shadow.glyph.contains("⚠️"));
        assert_eq!(shadow.color, ShadowColor::Red);
    }
}
