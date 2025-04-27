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

impl PagerWidget {
    pub fn new(items: &Vec<String>, height: usize, app_state: &mut AppState) -> Self {
        let scroll_off = app_state.config.scroll_off;
        let mut index = app_state.list_state.selected().unwrap_or(0);
        if items.len() == 0 {
            return Self::default();
        }
        if index >= items.len() {
            index = items.len() - 1;
        }
        app_state.list_state.select(Some(index));

        let mut offset = app_state.list_state.offset();
        // increase offset
        if index >= offset
            && height >= (scroll_off + 1)
            && index - offset > height - (scroll_off + 1)
        {
            offset = index + (scroll_off + 1) - height;
            if items.len() >= height && offset > items.len() - height {
                offset = items.len() - height;
            }
        }
        // reduce offset
        if offset + scroll_off >= index {
            offset = if scroll_off <= index {
                index - scroll_off
            } else {
                0
            }
        }
        *app_state.list_state.offset_mut() = offset;

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
