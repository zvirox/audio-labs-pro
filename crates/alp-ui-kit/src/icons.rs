//! The Audio Labs Pro Lucide icon set.
//!
//! Icons ship as embedded Lucide SVGs and are resolved at compile time with
//! [`lucide!`], so only the glyphs the app names end up in the binary. Every
//! control that used to draw a Unicode glyph now draws one of these, which
//! keeps stroke weight consistent in dark and light mode at any size.
//!
//! Lucide icons are ISC licensed; see `THIRD_PARTY.md`.
//!
//! Render an icon with [`tinted`] when the colour is known up front, or
//! [`themed`] to resolve it from the active theme on every draw.

pub use iced_cube::icon::{Glyph, Icon, themed, tinted};

macro_rules! icons {
    ($($(#[$meta:meta])* $name:ident => $lucide:ident;)*) => {
        $(
            $(#[$meta])*
            pub const $name: Glyph = iced_cube::lucide!($lucide);
        )*
    };
}

icons! {
    /// Transport and playback.
    PLAY => Play;
    PAUSE => Pause;
    STOP => Square;
    RECORD => Circle;
    SKIP_BACK => SkipBack;
    SKIP_FORWARD => SkipForward;
    LOOP => Repeat;
    VOLUME => Volume2;
    /// Generate workspace.
    GENERATE => Sparkles;
    MIC => Mic;
    WAND => WandSparkles;
    VOICES => Users;
    SEND => Send;
    /// Voices and models.
    LIBRARY => Library;
    MODELS => Boxes;
    GRID => LayoutGrid;
    ROWS => Rows3;
    /// Captures and stories.
    CAPTURES => Disc3;
    STORIES => Clapperboard;
    HEADPHONES => Headphones;
    /// Effects and AI.
    EFFECTS => Zap;
    AI => Bot;
    WAVEFORM => AudioWaveform;
    ACTIVITY => Activity;
    SIGMA => Sigma;
    /// Mixer and settings.
    MIXER => SlidersHorizontal;
    SETTINGS => Settings;
    /// Editing.
    TRIM => Scissors;
    DELETE_KEY => Delete;
    TRASH => Trash;
    PENCIL => Pencil;
    COPY => Copy;
    RESET => RotateCcw;
    MINUS => Minus;
    GRIP => GripVertical;
    /// Window and cards.
    CLOSE => X;
    CHECK => Check;
    PLUS => Plus;
    SEARCH => Search;
    MORE => Ellipsis;
    TARGET => Target;
    EXPAND => Maximize2;
    COLLAPSE => Minimize2;
    /// Directional.
    ARROW_UP => ArrowUp;
    ARROW_DOWN => ArrowDown;
    CHEVRON_UP => ChevronUp;
    CHEVRON_DOWN => ChevronDown;
    CHEVRON_LEFT => ChevronLeft;
    CHEVRON_RIGHT => ChevronRight;
    /// Files and system.
    UPLOAD => Upload;
    DOWNLOAD => Download;
    OPEN => ExternalLink;
    FOLDER => FolderOpen;
    STORAGE => HardDrive;
    SAVE => Save;
    SHARE => Share2;
    /// Preferences and feedback.
    CONTRAST => Contrast;
    THEME => SunMoon;
    KEYBOARD => Keyboard;
    PALETTE => Palette;
    INFO => Info;
    WARNING => TriangleAlert;
    SUCCESS => CircleCheck;
    FAILURE => CircleX;
    CLOCK => Clock;
    PIN => Pin;
    EYE => Eye;
    STAR => Star;
    HEART => Heart;
    COMMAND => Command;
    LAYERS => Layers;
    REFRESH => RefreshCw;
    FILE_TEXT => FileText;
}
