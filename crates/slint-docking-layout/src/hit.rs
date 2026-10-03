//! Hit-testing: from a point to the target a drop there would have.

use crate::geometry::{Point, Rect};
use crate::ops::{Target, Zone};
use crate::solve::{Metrics, Solved, SolvedGroup};
use crate::tree::{Layout, Node};

/// What a drop at a point would do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Where the panel would go.
    pub target: Target,
    /// The area to highlight while the pointer is there. For a side of a group or of a
    /// host this is the area the panel would take. For a place among the tabs it is a
    /// thin mark between two tabs.
    pub preview: Rect,
}

impl Layout {
    /// The target of a drop at `point` in a host, for a panel that is being dragged.
    ///
    /// `solved` must come from [`Layout::solve`] for the same host. Returns `None` when
    /// the point is outside the host.
    ///
    /// In order:
    ///
    /// 1. An empty host takes the panel whole.
    /// 2. A strip near an edge of the host is a drop on that edge, unless the host holds a
    ///    single group, where it would do the same as the side of the group.
    /// 3. The tabs of a group are a place among the tabs.
    /// 4. The rest of a group is the side nearest the point, or the centre.
    pub fn target_at(&self, host: usize, solved: &Solved, point: Point, metrics: &Metrics) -> Option<Hit> {
        let area = Rect::new(0.0, 0.0, solved.size.width, solved.size.height);
        if !area.contains(point) {
            return None;
        }
        let Some(root) = self.root(host) else {
            return Some(Hit { target: Target::Edge { host, zone: Zone::Center }, preview: area });
        };

        if matches!(root, Node::Split(_))
            && let Some(zone) = edge_zone(area, point, metrics.edge_band)
        {
            let preview = side(area, zone, 0.25);
            return Some(Hit { target: Target::Edge { host, zone }, preview });
        }

        let group = solved.groups.iter().find(|g| g.rect.contains(point))?;
        if group.tab_bar.contains(point) {
            return Some(tab_hit(group, point));
        }
        let zone = group_zone(group.content, point, metrics.zone_fraction);
        let preview = match zone {
            Zone::Center => group.rect,
            zone => side(group.rect, zone, 0.5),
        };
        Some(Hit { target: Target::Group { group: group.id, zone }, preview })
    }
}

fn tab_hit(group: &SolvedGroup, point: Point) -> Hit {
    let tabs = group.panels.len();
    let index = if group.tab_width > 0.0 {
        (((point.x - group.tab_bar.x) / group.tab_width).round() as usize).min(tabs)
    } else {
        tabs
    };
    let mark = 3.0;
    let x = group.tab_bar.x + index as f32 * group.tab_width - mark / 2.0;
    let x = x.clamp(group.tab_bar.x, (group.tab_bar.right() - mark).max(group.tab_bar.x));
    Hit {
        target: Target::Tab { group: group.id, index },
        preview: Rect::new(x, group.tab_bar.y, mark, group.tab_bar.height),
    }
}

/// The edge of `area` that `point` is within `band` of. The nearest edge wins.
fn edge_zone(area: Rect, point: Point, band: f32) -> Option<Zone> {
    nearest(area, point).filter(|(_, distance)| *distance < band).map(|(zone, _)| zone)
}

/// The zone of a group that `point` is in.
fn group_zone(area: Rect, point: Point, fraction: f32) -> Zone {
    if area.width <= 0.0 || area.height <= 0.0 {
        return Zone::Center;
    }
    // Compare distances as shares of the area, so that a wide group has wide side zones.
    let shares = [
        (Zone::Left, (point.x - area.x) / area.width),
        (Zone::Right, (area.right() - point.x) / area.width),
        (Zone::Top, (point.y - area.y) / area.height),
        (Zone::Bottom, (area.bottom() - point.y) / area.height),
    ];
    shares
        .into_iter()
        .filter(|(_, share)| *share < fraction)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map_or(Zone::Center, |(zone, _)| zone)
}

fn nearest(area: Rect, point: Point) -> Option<(Zone, f32)> {
    [
        (Zone::Left, point.x - area.x),
        (Zone::Right, area.right() - point.x),
        (Zone::Top, point.y - area.y),
        (Zone::Bottom, area.bottom() - point.y),
    ]
    .into_iter()
    .min_by(|a, b| a.1.total_cmp(&b.1))
}

/// The part of `area` on one side, `share` of it wide or high.
fn side(area: Rect, zone: Zone, share: f32) -> Rect {
    let (w, h) = (area.width * share, area.height * share);
    match zone {
        Zone::Center => area,
        Zone::Left => Rect::new(area.x, area.y, w, area.height),
        Zone::Right => Rect::new(area.right() - w, area.y, w, area.height),
        Zone::Top => Rect::new(area.x, area.y, area.width, h),
        Zone::Bottom => Rect::new(area.x, area.bottom() - h, area.width, h),
    }
}
