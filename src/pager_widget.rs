use std::cmp::min;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::Text,
    widgets::{Block, Borders, List, ListItem, ListState, StatefulWidget},
};

use crate::{app_state::AppState, ui::highlight_style};
use ansi_to_tui::IntoText as _;

#[derive(Clone)]
pub struct PagerWidget {
    inner: List<'static>,
    state: ListState,
}

// duplicates some logic
pub fn adapt_index_in_frame(
    offset: usize,
    scrolloff: usize,
    mut index: usize,
    height: usize,
    len: usize
) -> usize {
    if offset + scrolloff >= index {
        index = offset + scrolloff;
    }
    if index >= len {
        index = len - 1;
    }
    if offset + height > scrolloff && index >= offset + height - scrolloff {
        index = offset + height - scrolloff - 1;
    }
    index
}

impl PagerWidget {
    pub fn new(
        items: &Vec<String>,
        height: usize,
        app_state: &mut AppState,
        scroll: Option<bool>,
        scroll_step : usize,
    ) -> Self {
        let scrolloff = app_state.config.scrolloff;

        // ensure the real index is properly defined
        let mut index = app_state.list_state.selected().unwrap_or(0);
        if index >= items.len() {
            index = items.len() - 1;
        }
        if items.len() == 0 {
            return Self::default();
        }
        let mut offset = app_state.list_state.offset();

        match scroll {
            None => {
                // move manually from the boundaries index state
                // increase offset
                if index >= offset
                    && height >= (scrolloff + 1)
                    && index - offset > height - (scrolloff + 1)
                {
                    offset = index + (scrolloff + 1) - height;
                    if items.len() >= height && offset > items.len() - height {
                        offset = items.len() - height;
                    }
                }
                // reduce offset
                if offset + scrolloff >= index {
                    offset = if scrolloff <= index {
                        index - scrolloff
                    } else {
                        0
                    }
                }
            },
            Some(down) => {
                match down {
                    true => {
                        offset += scroll_step;
                        if items.len() >= scrolloff + 1 && offset >= items.len() - scrolloff - 1 {
                            offset = items.len() - scrolloff - 1
                        }
                    },
                    false => {
                        if offset < scroll_step {
                            offset = 0;
                        } else {
                            offset = offset - scroll_step;
                        }
                    },
                };
                index = adapt_index_in_frame(offset, scrolloff, index, height, items.len());
            }
        }
        *app_state.list_state.offset_mut() = offset;
        app_state.list_state.select(Some(index));


        let first = app_state.list_state.offset();
        let last = min(first + height, items.len());

        let mut state = ListState::default();
        if index >= first {
            state.select(Some(index - first));
        }

        let list_items: Vec<ListItem> = items[first..last]
            .into_iter()
            .map(|s| {
                let text = s.as_bytes().into_text().unwrap_or(Text::default());
                return ListItem::new(text);
            })
            .collect();
        let inner = List::new(list_items)
            .block(Block::default().borders(Borders::NONE))
            .highlight_style(highlight_style());
        Self { inner, state }
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(&self.inner, area, buf, &mut self.state);
    }
}

impl Default for PagerWidget {
    fn default() -> Self {
        Self {
            inner: List::default(),
            state: ListState::default(),
        }
    }
}
