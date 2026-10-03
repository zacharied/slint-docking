//! Points, sizes and rectangles, in logical pixels.

use crate::Axis;

/// A position in a host, measured from its top left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Distance from the left edge.
    pub x: f32,
    /// Distance from the top edge.
    pub y: f32,
}

impl Point {
    /// A point at the given position.
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// The coordinate that runs along `axis`.
    pub fn along(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.x,
            Axis::Vertical => self.y,
        }
    }
}

/// The size of a host or of anything in it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Size {
    /// Extent from left to right.
    pub width: f32,
    /// Extent from top to bottom.
    pub height: f32,
}

impl Size {
    /// A size of the given extents.
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

/// A rectangle in a host.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// Left edge.
    pub x: f32,
    /// Top edge.
    pub y: f32,
    /// Extent from left to right.
    pub width: f32,
    /// Extent from top to bottom.
    pub height: f32,
}

impl Rect {
    /// A rectangle with the given corner and extents.
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }

    /// The right edge.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// The bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    /// The point in the middle.
    pub fn center(&self) -> Point {
        Point::new(self.x + self.width / 2.0, self.y + self.height / 2.0)
    }

    /// Whether `point` lies inside. The left and top edges are inside, the right and bottom
    /// edges are not, so that rectangles which tile an area never both contain a point.
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x < self.right() && point.y >= self.y && point.y < self.bottom()
    }

    /// The start and the extent along `axis`.
    pub fn span(&self, axis: Axis) -> (f32, f32) {
        match axis {
            Axis::Horizontal => (self.x, self.width),
            Axis::Vertical => (self.y, self.height),
        }
    }

    /// This rectangle with its span along `axis` replaced.
    pub fn with_span(&self, axis: Axis, start: f32, extent: f32) -> Rect {
        match axis {
            Axis::Horizontal => Rect::new(start, self.y, extent, self.height),
            Axis::Vertical => Rect::new(self.x, start, self.width, extent),
        }
    }

    /// The area covered.
    pub fn area(&self) -> f32 {
        self.width * self.height
    }
}
