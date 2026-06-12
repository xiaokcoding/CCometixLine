mod composition;
mod filter;
mod join;
mod separator;
mod width;

pub use composition::{render_segment, CompositionPhase};
pub use filter::FilterPhase;
pub use join::JoinPhase;
pub use separator::{powerline_arrow, SeparatorPhase, POWERLINE_ARROW};
pub use width::{terminal_width, truncate_to_width, WidthPhase};
