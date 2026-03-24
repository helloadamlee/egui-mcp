use egui::{Color32, Rect, Shape, Stroke};

pub struct Highlight {
    pub rect: Rect,
    pub color: Color32,
    pub thickness: f32,
}

impl Highlight {
    pub fn new(rect: Rect, color: Color32, thickness: f32) -> Self {
        Self {
            rect,
            color,
            thickness,
        }
    }

    pub fn draw(&self, shapes: &mut Vec<Shape>) {
        shapes.push(Shape::rect_stroke(
            self.rect,
            4.0,
            Stroke::new(self.thickness, self.color),
        ));
    }
}

pub struct HighlightManager {
    highlights: Vec<Highlight>,
}

impl HighlightManager {
    pub fn new() -> Self {
        Self {
            highlights: Vec::new(),
        }
    }

    pub fn add(&mut self, highlight: Highlight) {
        self.highlights.push(highlight);
    }

    pub fn remove(&mut self, index: usize) {
        self.highlights.remove(index);
    }

    pub fn clear(&mut self) {
        self.highlights.clear();
    }

    pub fn draw(&self, shapes: &mut Vec<Shape>) {
        for highlight in &self.highlights {
            highlight.draw(shapes);
        }
    }
}
