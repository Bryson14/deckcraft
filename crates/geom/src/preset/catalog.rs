use super::builder::vis;
use super::{BezPath, Deserialize, Point, Rect, Serialize, Vec2};

/// How a sub-path is filled. `Lighten`/`Darken` variants shade the shape's fill (3-D looking faces).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FillMode {
    Norm,
    None,
    Lighten,
    LightenLess,
    Darken,
    DarkenLess,
}

#[derive(Clone, Debug)]
pub struct SubPath {
    pub path: BezPath,
    pub fill: FillMode,
    pub stroke: bool,
    /// Even-odd fill (holes in donuts and frames).
    pub even_odd: bool,
}

/// A draggable adjust handle (the yellow dot). `pos = origin + dir * value`; dragging projects
/// the pointer onto `dir`. An `angle` handle instead measures the pointer's angle about `origin`
/// (1/60 000 degree, y down, 0° at +x).
#[derive(Clone, Debug, PartialEq)]
pub struct Handle {
    pub index: usize,
    pub pos: Point,
    pub origin: Point,
    pub dir: Vec2,
    pub min: f64,
    pub max: f64,
    pub angle: bool,
}

impl Handle {
    pub(super) fn new(index: usize, origin: Point, dir: Vec2, value: f64, min: f64, max: f64) -> Self {
        Handle { index, pos: origin + dir * value, origin, dir, min, max, angle: false }
    }
    /// An angle handle on the `rx`×`ry` ellipse around `center` at visual angle `value`.
    pub(super) fn angle(index: usize, center: Point, rx: f64, ry: f64, value: f64) -> Self {
        let t = vis(value / 60000.0, rx, ry).to_radians();
        let pos = Point::new(center.x + rx * t.cos(), center.y + ry * t.sin());
        Handle { index, pos, origin: center, dir: Vec2::ZERO, min: 0.0, max: 21600000.0, angle: true }
    }
    /// The adjust value for a pointer at `p` (shape-local space).
    pub fn value_at(&self, p: Point) -> f64 {
        if self.angle {
            let d = p - self.origin;
            if !(d.x.is_finite() && d.y.is_finite()) || d.hypot() < 1e-9 {
                return self.min;
            }
            let v = (d.y.atan2(d.x).to_degrees().rem_euclid(360.0) * 60000.0).round();
            return if v >= self.max { self.min } else { v };
        }
        let l2 = self.dir.hypot2();
        if l2 < 1e-18 {
            return self.min;
        }
        let v = (p - self.origin).dot(self.dir) / l2;
        if !v.is_finite() {
            return self.min;
        }
        v.clamp(self.min.min(self.max), self.max.max(self.min)).round()
    }
}

#[derive(Clone, Debug)]
pub struct Geometry {
    pub paths: Vec<SubPath>,
    /// Where text goes, in shape-local space.
    pub text_rect: Rect,
    pub handles: Vec<Handle>,
    /// Connector glue points, shape-local.
    pub sites: Vec<Point>,
}

impl Geometry {
    /// The union outline used for hit testing and selection (all filled or stroked sub-paths).
    pub fn outline(&self) -> BezPath {
        let mut p = BezPath::new();
        for s in &self.paths {
            p.extend(s.path.iter());
        }
        p
    }
    /// Whether nothing is filled (lines, connectors, arcs, brackets).
    pub fn is_open(&self) -> bool {
        self.paths.iter().all(|s| s.fill == FillMode::None)
    }
}

/// Gallery category, as in the Shapes menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Category {
    Lines,
    Rectangles,
    BasicShapes,
    BlockArrows,
    EquationShapes,
    Flowchart,
    StarsAndBanners,
    Callouts,
    ActionButtons,
}

impl Category {
    pub const ALL: [Category; 9] = [
        Category::Lines,
        Category::Rectangles,
        Category::BasicShapes,
        Category::BlockArrows,
        Category::EquationShapes,
        Category::Flowchart,
        Category::StarsAndBanners,
        Category::Callouts,
        Category::ActionButtons,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Category::Lines => "Lines",
            Category::Rectangles => "Rectangles",
            Category::BasicShapes => "Basic Shapes",
            Category::BlockArrows => "Block Arrows",
            Category::EquationShapes => "Equation Shapes",
            Category::Flowchart => "Flowchart",
            Category::StarsAndBanners => "Stars and Banners",
            Category::Callouts => "Callouts",
            Category::ActionButtons => "Action Buttons",
        }
    }
}

/// Every preset we generate, one variant per Office Open XML name, in gallery order. The macro
/// expands the variant list into the `Preset` enum, its name table, and the catalog entries.
macro_rules! presets {
    ($($v:ident, $n:literal, $l:literal, $c:ident, [$($d:expr),*];)* ; $($dv:ident, $dn:literal;)*) => {
        /// A preset shape, named by its variant. `Preset::from_ooxml` parses the file's name.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum Preset {
            $($v,)*
            $($dv,)*
        }

        impl Preset {
            /// Every preset, in gallery order (the gallery's presets first, then the names that only
            /// files use).
            pub const ALL: &'static [Preset] = &[$(Preset::$v,)* $(Preset::$dv,)*];

            /// The preset's Office Open XML name (`rect`, `roundRect`, `rightArrow`…).
            pub const fn ooxml(self) -> &'static str {
                match self {
                    $(Preset::$v => $n,)*
                    $(Preset::$dv => $dn,)*
                }
            }

            /// Parses an Office Open XML preset name; `None` for any name we don't build.
            pub fn from_ooxml(name: &str) -> Option<Preset> {
                match name {
                    $($n => Some(Preset::$v),)*
                    $($dn => Some(Preset::$dv),)*
                    _ => None,
                }
            }

            /// The preset's catalog entry (label, category, default adjust values); `None` for the
            /// names that only files use and the gallery does not show.
            pub fn info(self) -> Option<&'static PresetInfo> {
                CATALOG.iter().find(|p| p.preset == self)
            }
        }

        /// Every preset we generate, in gallery order.
        pub static CATALOG: &[PresetInfo] = &[
            $(PresetInfo { preset: Preset::$v, name: $n, label: $l, category: Category::$c, defaults: &[$($d as f64),*] },)*
        ];
    };
}

pub struct PresetInfo {
    pub preset: Preset,
    pub name: &'static str,
    pub label: &'static str,
    pub category: Category,
    /// Default adjust values.
    pub defaults: &'static [f64],
}

presets! {
    Line, "line", "Line", Lines, [];
    StraightConnector1, "straightConnector1", "Arrow", Lines, [];
    BentConnector3, "bentConnector3", "Connector: Elbow", Lines, [50000];
    CurvedConnector3, "curvedConnector3", "Connector: Curved", Lines, [50000];
    BentConnector2, "bentConnector2", "Connector: Elbow (one bend)", Lines, [];
    LineInv, "lineInv", "Line: Rising", Lines, [];
    Rect, "rect", "Rectangle", Rectangles, [];
    RoundRect, "roundRect", "Rectangle: Rounded Corners", Rectangles, [16667];
    Snip1Rect, "snip1Rect", "Rectangle: Single Corner Snipped", Rectangles, [16667];
    Snip2SameRect, "snip2SameRect", "Rectangle: Top Corners Snipped", Rectangles, [16667, 0];
    Snip2DiagRect, "snip2DiagRect", "Rectangle: Diagonal Corners Snipped", Rectangles, [0, 16667];
    SnipRoundRect, "snipRoundRect", "Rectangle: Top Corners One Rounded and One Snipped", Rectangles, [16667, 16667];
    Round1Rect, "round1Rect", "Rectangle: Single Corner Rounded", Rectangles, [16667];
    Round2SameRect, "round2SameRect", "Rectangle: Top Corners Rounded", Rectangles, [16667, 0];
    Round2DiagRect, "round2DiagRect", "Rectangle: Diagonal Corners Rounded", Rectangles, [16667, 0];
    TextBox, "textBox", "Text Box", BasicShapes, [];
    Ellipse, "ellipse", "Oval", BasicShapes, [];
    Triangle, "triangle", "Isosceles Triangle", BasicShapes, [50000];
    RtTriangle, "rtTriangle", "Right Triangle", BasicShapes, [];
    Parallelogram, "parallelogram", "Parallelogram", BasicShapes, [25000];
    Trapezoid, "trapezoid", "Trapezoid", BasicShapes, [25000];
    NonIsoscelesTrapezoid, "nonIsoscelesTrapezoid", "Trapezoid: Uneven Sides", BasicShapes, [25000, 25000];
    Diamond, "diamond", "Diamond", BasicShapes, [];
    Pentagon, "pentagon", "Regular Pentagon", BasicShapes, [];
    Hexagon, "hexagon", "Hexagon", BasicShapes, [25000];
    Heptagon, "heptagon", "Heptagon", BasicShapes, [];
    Octagon, "octagon", "Octagon", BasicShapes, [29289];
    Decagon, "decagon", "Decagon", BasicShapes, [];
    Dodecagon, "dodecagon", "Dodecagon", BasicShapes, [];
    Pie, "pie", "Partial Circle", BasicShapes, [0, 16200000];
    PieWedge, "pieWedge", "Quarter Wedge", BasicShapes, [];
    Chord, "chord", "Chord", BasicShapes, [2700000, 16200000];
    Teardrop, "teardrop", "Teardrop", BasicShapes, [100000];
    Frame, "frame", "Frame", BasicShapes, [12500];
    CornerTabs, "cornerTabs", "Corner Tabs: Triangles", BasicShapes, [];
    SquareTabs, "squareTabs", "Corner Tabs: Squares", BasicShapes, [];
    PlaqueTabs, "plaqueTabs", "Corner Tabs: Quarter Circles", BasicShapes, [];
    HalfFrame, "halfFrame", "Half Frame", BasicShapes, [33333, 33333];
    Corner, "corner", "L-Shape", BasicShapes, [50000, 50000];
    DiagStripe, "diagStripe", "Diagonal Stripe", BasicShapes, [50000];
    Plus, "plus", "Cross", BasicShapes, [25000];
    Plaque, "plaque", "Plaque", BasicShapes, [16667];
    Can, "can", "Cylinder", BasicShapes, [25000];
    Cube, "cube", "Cube", BasicShapes, [25000];
    Bevel, "bevel", "Bevel", BasicShapes, [12500];
    Donut, "donut", "Donut", BasicShapes, [25000];
    NoSmoking, "noSmoking", "\"Not Allowed\" Symbol", BasicShapes, [18750];
    BlockArc, "blockArc", "Block Arc", BasicShapes, [10800000, 0, 25000];
    FoldedCorner, "foldedCorner", "Folded Corner", BasicShapes, [16667];
    SmileyFace, "smileyFace", "Smiley Face", BasicShapes, [4653];
    Heart, "heart", "Heart", BasicShapes, [];
    LightningBolt, "lightningBolt", "Lightning Bolt", BasicShapes, [];
    Sun, "sun", "Sun", BasicShapes, [25000];
    Moon, "moon", "Moon", BasicShapes, [50000];
    Cloud, "cloud", "Cloud", BasicShapes, [];
    Gear6, "gear6", "Gear: 6 Teeth", BasicShapes, [15000, 3526];
    Gear9, "gear9", "Gear: 9 Teeth", BasicShapes, [10000, 1763];
    Funnel, "funnel", "Funnel", BasicShapes, [];
    Arc, "arc", "Arc", BasicShapes, [16200000, 0];
    BracketPair, "bracketPair", "Double Bracket", BasicShapes, [16667];
    BracePair, "bracePair", "Double Brace", BasicShapes, [8333];
    LeftBracket, "leftBracket", "Left Bracket", BasicShapes, [8333];
    RightBracket, "rightBracket", "Right Bracket", BasicShapes, [8333];
    LeftBrace, "leftBrace", "Left Brace", BasicShapes, [8333, 50000];
    RightBrace, "rightBrace", "Right Brace", BasicShapes, [8333, 50000];
    ChartX, "chartX", "Chart Marker: Cross Lines", BasicShapes, [];
    ChartStar, "chartStar", "Chart Marker: Star Lines", BasicShapes, [];
    ChartPlus, "chartPlus", "Chart Marker: Plus Lines", BasicShapes, [];
    RightArrow, "rightArrow", "Arrow: Right", BlockArrows, [50000, 50000];
    LeftArrow, "leftArrow", "Arrow: Left", BlockArrows, [50000, 50000];
    UpArrow, "upArrow", "Arrow: Up", BlockArrows, [50000, 50000];
    DownArrow, "downArrow", "Arrow: Down", BlockArrows, [50000, 50000];
    LeftRightArrow, "leftRightArrow", "Arrow: Left-Right", BlockArrows, [50000, 50000];
    UpDownArrow, "upDownArrow", "Arrow: Up-Down", BlockArrows, [50000, 50000];
    QuadArrow, "quadArrow", "Arrow: Quad", BlockArrows, [22500, 22500, 22500];
    LeftRightUpArrow, "leftRightUpArrow", "Arrow: Left-Right-Up", BlockArrows, [25000, 25000, 25000];
    BentArrow, "bentArrow", "Arrow: Bent", BlockArrows, [25000, 25000, 25000, 43750];
    UturnArrow, "uturnArrow", "Arrow: U-Turn", BlockArrows, [25000, 25000, 25000, 43750, 75000];
    CurvedRightArrow, "curvedRightArrow", "Arrow: Curved Right", BlockArrows, [25000, 50000, 25000];
    CurvedLeftArrow, "curvedLeftArrow", "Arrow: Curved Left", BlockArrows, [25000, 50000, 25000];
    CurvedUpArrow, "curvedUpArrow", "Arrow: Curved Up", BlockArrows, [25000, 50000, 25000];
    CurvedDownArrow, "curvedDownArrow", "Arrow: Curved Down", BlockArrows, [25000, 50000, 25000];
    LeftUpArrow, "leftUpArrow", "Arrow: Left-Up", BlockArrows, [25000, 25000, 25000];
    BentUpArrow, "bentUpArrow", "Arrow: Bent-Up", BlockArrows, [25000, 25000, 25000];
    StripedRightArrow, "stripedRightArrow", "Arrow: Striped Right", BlockArrows, [50000, 50000];
    NotchedRightArrow, "notchedRightArrow", "Arrow: Notched Right", BlockArrows, [50000, 50000];
    HomePlate, "homePlate", "Arrow: Pentagon", BlockArrows, [50000];
    Chevron, "chevron", "Arrow: Chevron", BlockArrows, [50000];
    CircularArrow, "circularArrow", "Arrow: Circular", BlockArrows, [12500, 1142319, 20457681, 10800000, 12500];
    LeftCircularArrow, "leftCircularArrow", "Arrow: Circular Counter-clockwise", BlockArrows, [12500, -1142319, 1142319, 10800000, 12500];
    LeftRightCircularArrow, "leftRightCircularArrow", "Arrow: Circular Two-headed", BlockArrows, [12500, 1142319, 20457681, 11942319, 12500];
    SwooshArrow, "swooshArrow", "Arrow: Swoosh", BlockArrows, [25000, 16667];
    RightArrowCallout, "rightArrowCallout", "Callout: Right Arrow", BlockArrows, [25000, 25000, 25000, 64977];
    DownArrowCallout, "downArrowCallout", "Callout: Down Arrow", BlockArrows, [25000, 25000, 25000, 64977];
    LeftArrowCallout, "leftArrowCallout", "Callout: Left Arrow", BlockArrows, [25000, 25000, 25000, 64977];
    UpArrowCallout, "upArrowCallout", "Callout: Up Arrow", BlockArrows, [25000, 25000, 25000, 64977];
    LeftRightArrowCallout, "leftRightArrowCallout", "Callout: Left-Right Arrow", BlockArrows, [25000, 25000, 25000, 48123];
    UpDownArrowCallout, "upDownArrowCallout", "Callout: Up-Down Arrow", BlockArrows, [25000, 25000, 25000, 48123];
    QuadArrowCallout, "quadArrowCallout", "Callout: Quad Arrow", BlockArrows, [18515, 18515, 18515, 48123];
    MathPlus, "mathPlus", "Plus Sign", EquationShapes, [23520];
    MathMinus, "mathMinus", "Minus Sign", EquationShapes, [23520];
    MathMultiply, "mathMultiply", "Multiplication Sign", EquationShapes, [23520];
    MathDivide, "mathDivide", "Division Sign", EquationShapes, [23520, 5880, 11760];
    MathEqual, "mathEqual", "Equals", EquationShapes, [23520, 11760];
    MathNotEqual, "mathNotEqual", "Not Equal", EquationShapes, [23520, 6600000, 11760];
    FlowChartProcess, "flowChartProcess", "Flowchart: Process", Flowchart, [];
    FlowChartAlternateProcess, "flowChartAlternateProcess", "Flowchart: Alternate Process", Flowchart, [];
    FlowChartDecision, "flowChartDecision", "Flowchart: Decision", Flowchart, [];
    FlowChartInputOutput, "flowChartInputOutput", "Flowchart: Data", Flowchart, [];
    FlowChartPredefinedProcess, "flowChartPredefinedProcess", "Flowchart: Predefined Process", Flowchart, [];
    FlowChartInternalStorage, "flowChartInternalStorage", "Flowchart: Internal Storage", Flowchart, [];
    FlowChartDocument, "flowChartDocument", "Flowchart: Document", Flowchart, [];
    FlowChartMultidocument, "flowChartMultidocument", "Flowchart: Multidocument", Flowchart, [];
    FlowChartTerminator, "flowChartTerminator", "Flowchart: Terminator", Flowchart, [];
    FlowChartPreparation, "flowChartPreparation", "Flowchart: Preparation", Flowchart, [];
    FlowChartManualInput, "flowChartManualInput", "Flowchart: Manual Input", Flowchart, [];
    FlowChartManualOperation, "flowChartManualOperation", "Flowchart: Manual Operation", Flowchart, [];
    FlowChartConnector, "flowChartConnector", "Flowchart: Connector", Flowchart, [];
    FlowChartOffpageConnector, "flowChartOffpageConnector", "Flowchart: Off-page Connector", Flowchart, [];
    FlowChartPunchedCard, "flowChartPunchedCard", "Flowchart: Card", Flowchart, [];
    FlowChartPunchedTape, "flowChartPunchedTape", "Flowchart: Punched Tape", Flowchart, [];
    FlowChartSummingJunction, "flowChartSummingJunction", "Flowchart: Summing Junction", Flowchart, [];
    FlowChartOr, "flowChartOr", "Flowchart: Or", Flowchart, [];
    FlowChartCollate, "flowChartCollate", "Flowchart: Collate", Flowchart, [];
    FlowChartSort, "flowChartSort", "Flowchart: Sort", Flowchart, [];
    FlowChartExtract, "flowChartExtract", "Flowchart: Extract", Flowchart, [];
    FlowChartMerge, "flowChartMerge", "Flowchart: Merge", Flowchart, [];
    FlowChartOnlineStorage, "flowChartOnlineStorage", "Flowchart: Stored Data", Flowchart, [];
    FlowChartDelay, "flowChartDelay", "Flowchart: Delay", Flowchart, [];
    FlowChartMagneticTape, "flowChartMagneticTape", "Flowchart: Sequential Access Storage", Flowchart, [];
    FlowChartMagneticDisk, "flowChartMagneticDisk", "Flowchart: Magnetic Disk", Flowchart, [];
    FlowChartMagneticDrum, "flowChartMagneticDrum", "Flowchart: Direct Access Storage", Flowchart, [];
    FlowChartDisplay, "flowChartDisplay", "Flowchart: Display", Flowchart, [];
    FlowChartOfflineStorage, "flowChartOfflineStorage", "Flowchart: Offline Storage", Flowchart, [];
    IrregularSeal1, "irregularSeal1", "Explosion: 8 Points", StarsAndBanners, [];
    IrregularSeal2, "irregularSeal2", "Explosion: 14 Points", StarsAndBanners, [];
    Star4, "star4", "Star: 4 Points", StarsAndBanners, [12500];
    Star5, "star5", "Star: 5 Points", StarsAndBanners, [19098];
    Star6, "star6", "Star: 6 Points", StarsAndBanners, [28868];
    Star7, "star7", "Star: 7 Points", StarsAndBanners, [34601];
    Star8, "star8", "Star: 8 Points", StarsAndBanners, [37500];
    Star10, "star10", "Star: 10 Points", StarsAndBanners, [42533];
    Star12, "star12", "Star: 12 Points", StarsAndBanners, [37500];
    Star16, "star16", "Star: 16 Points", StarsAndBanners, [37500];
    Star24, "star24", "Star: 24 Points", StarsAndBanners, [37500];
    Star32, "star32", "Star: 32 Points", StarsAndBanners, [37500];
    Ribbon2, "ribbon2", "Ribbon: Tilted Up", StarsAndBanners, [16667, 50000];
    Ribbon, "ribbon", "Ribbon: Tilted Down", StarsAndBanners, [16667, 50000];
    EllipseRibbon2, "ellipseRibbon2", "Ribbon: Curved and Tilted Up", StarsAndBanners, [25000, 50000, 12500];
    EllipseRibbon, "ellipseRibbon", "Ribbon: Curved and Tilted Down", StarsAndBanners, [25000, 50000, 12500];
    LeftRightRibbon, "leftRightRibbon", "Ribbon: Left-Right Fold", StarsAndBanners, [50000, 50000, 16667];
    VerticalScroll, "verticalScroll", "Scroll: Vertical", StarsAndBanners, [12500];
    HorizontalScroll, "horizontalScroll", "Scroll: Horizontal", StarsAndBanners, [12500];
    Wave, "wave", "Wave", StarsAndBanners, [12500, 0];
    DoubleWave, "doubleWave", "Double Wave", StarsAndBanners, [6250, 0];
    WedgeRectCallout, "wedgeRectCallout", "Speech Bubble: Rectangle", Callouts, [-20833, 62500];
    WedgeRoundRectCallout, "wedgeRoundRectCallout", "Speech Bubble: Rectangle with Corners Rounded", Callouts, [-20833, 62500, 16667];
    WedgeEllipseCallout, "wedgeEllipseCallout", "Speech Bubble: Oval", Callouts, [-20833, 62500];
    CloudCallout, "cloudCallout", "Thought Bubble: Cloud", Callouts, [-20833, 62500];
    BorderCallout1, "borderCallout1", "Callout: Line", Callouts, [18750, -8333, 112500, -38333];
    BorderCallout2, "borderCallout2", "Callout: Bent Line", Callouts, [18750, -8333, 18750, -16667, 112500, -46667];
    BorderCallout3, "borderCallout3", "Callout: Double Bent Line", Callouts, [18750, -8333, 18750, -16667, 100000, -16667, 112963, -8333];
    AccentCallout1, "accentCallout1", "Callout: Line with Accent Bar", Callouts, [18750, -8333, 112500, -38333];
    AccentCallout2, "accentCallout2", "Callout: Bent Line with Accent Bar", Callouts, [18750, -8333, 18750, -16667, 112500, -46667];
    AccentCallout3, "accentCallout3", "Callout: Double Bent Line with Accent Bar", Callouts, [18750, -8333, 18750, -16667, 100000, -16667, 112963, -8333];
    Callout1, "callout1", "Callout: Line with No Border", Callouts, [18750, -8333, 112500, -38333];
    Callout2, "callout2", "Callout: Bent Line with No Border", Callouts, [18750, -8333, 18750, -16667, 112500, -46667];
    Callout3, "callout3", "Callout: Double Bent Line with No Border", Callouts, [18750, -8333, 18750, -16667, 100000, -16667, 112963, -8333];
    AccentBorderCallout1, "accentBorderCallout1", "Callout: Line with Border and Accent Bar", Callouts, [18750, -8333, 112500, -38333];
    AccentBorderCallout2, "accentBorderCallout2", "Callout: Bent Line with Border and Accent Bar", Callouts, [18750, -8333, 18750, -16667, 112500, -46667];
    AccentBorderCallout3, "accentBorderCallout3", "Callout: Double Bent Line with Border and Accent Bar", Callouts, [18750, -8333, 18750, -16667, 100000, -16667, 112963, -8333];
    ActionButtonBackPrevious, "actionButtonBackPrevious", "Action Button: Go Back or Previous", ActionButtons, [];
    ActionButtonForwardNext, "actionButtonForwardNext", "Action Button: Go Forward or Next", ActionButtons, [];
    ActionButtonBeginning, "actionButtonBeginning", "Action Button: Go to Beginning", ActionButtons, [];
    ActionButtonEnd, "actionButtonEnd", "Action Button: Go to End", ActionButtons, [];
    ActionButtonHome, "actionButtonHome", "Action Button: Go Home", ActionButtons, [];
    ActionButtonInformation, "actionButtonInformation", "Action Button: Get Information", ActionButtons, [];
    ActionButtonReturn, "actionButtonReturn", "Action Button: Go Back", ActionButtons, [];
    ActionButtonMovie, "actionButtonMovie", "Action Button: Video", ActionButtons, [];
    ActionButtonDocument, "actionButtonDocument", "Action Button: Document", ActionButtons, [];
    ActionButtonSound, "actionButtonSound", "Action Button: Sound", ActionButtons, [];
    ActionButtonHelp, "actionButtonHelp", "Action Button: Help", ActionButtons, [];
    ActionButtonBlank, "actionButtonBlank", "Action Button: Custom", ActionButtons, [];
    ;
    BentConnector4, "bentConnector4";
    BentConnector5, "bentConnector5";
    CurvedConnector2, "curvedConnector2";
    CurvedConnector4, "curvedConnector4";
    CurvedConnector5, "curvedConnector5";
}

pub fn info(name: &str) -> Option<&'static PresetInfo> {
    Preset::from_ooxml(name)?.info()
}

impl Preset {
    /// Whether PowerPoint joins this preset's outline with sharp (miter) corners when the line names
    /// no join (it rounds them otherwise). Observed per preset on its export: the plus, the right,
    /// up and down block arrows (not the left one), chevrons, stars from five points up, the
    /// equation shapes, curved ribbons and the action buttons are mitred.
    pub fn mitred_by_default(self) -> bool {
        matches!(
            self,
            Preset::Plus
                | Preset::RightArrow
                | Preset::UpArrow
                | Preset::DownArrow
                | Preset::UpDownArrow
                | Preset::LeftUpArrow
                | Preset::BentUpArrow
                | Preset::StripedRightArrow
                | Preset::NotchedRightArrow
                | Preset::Chevron
                | Preset::LeftArrowCallout
                | Preset::LeftRightArrowCallout
                | Preset::UpDownArrowCallout
                | Preset::QuadArrowCallout
                | Preset::MathPlus
                | Preset::MathMinus
                | Preset::MathMultiply
                | Preset::MathEqual
                | Preset::MathNotEqual
                | Preset::Star5
                | Preset::Star6
                | Preset::Star8
                | Preset::Star10
                | Preset::Star12
                | Preset::Star16
                | Preset::Star24
                | Preset::Star32
                | Preset::EllipseRibbon
                | Preset::EllipseRibbon2
                | Preset::ActionButtonBackPrevious
                | Preset::ActionButtonForwardNext
                | Preset::ActionButtonBeginning
                | Preset::ActionButtonEnd
                | Preset::ActionButtonHome
                | Preset::ActionButtonInformation
                | Preset::ActionButtonHelp
                | Preset::ActionButtonReturn
                | Preset::ActionButtonMovie
                | Preset::ActionButtonDocument
                | Preset::ActionButtonSound
                | Preset::ActionButtonBlank
        )
    }

    /// Whether the preset is a line or connector (its outline is open, not filled).
    pub fn is_line_like(self) -> bool {
        matches!(
            self,
            Preset::Line
                | Preset::StraightConnector1
                | Preset::BentConnector2
                | Preset::BentConnector3
                | Preset::CurvedConnector3
                | Preset::Arc
                | Preset::BentConnector4
                | Preset::BentConnector5
                | Preset::CurvedConnector2
                | Preset::CurvedConnector4
                | Preset::CurvedConnector5
        )
    }
}

/// Whether PowerPoint joins this preset's outline with sharp corners by default; see
/// [`Preset::mitred_by_default`]. Unknown names are not mitred.
pub fn mitred_by_default(name: &str) -> bool {
    Preset::from_ooxml(name).is_some_and(Preset::mitred_by_default)
}

/// Whether the named preset is a line or connector; see [`Preset::is_line_like`].
pub fn is_line_like(name: &str) -> bool {
    Preset::from_ooxml(name).is_some_and(Preset::is_line_like)
}
