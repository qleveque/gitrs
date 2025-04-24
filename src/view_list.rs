use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, List, ListItem, ListState, StatefulWidget},
};

pub trait ToListItem: Clone {
    fn to_list_item(t: Self) -> ListItem<'static>;
}

#[derive(Clone)]
pub struct ViewList {
    inner: List<'static>,
}

#[allow(dead_code)]
impl ViewList {
    pub fn new<T>(
        items: &Vec<T>,
        block: Block<'static>,
        style: Style,
        highlight_style: Style,
        scroll_off: usize,
    ) -> ViewList
    where
        T: ToListItem,
    {
        let list_items: Vec<ListItem> = items
            .into_iter()
            .map(|t| T::to_list_item(t.clone()))
            .collect();
        let inner = List::new(list_items)
            .block(block)
            .style(style)
            .highlight_style(highlight_style)
            .scroll_padding(scroll_off);
        ViewList { inner }
    }
}

impl StatefulWidget for ViewList {
    type State = ListState;
    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        self.inner.render(area, buf, state);
    }
}

impl Default for ViewList {
    fn default() -> Self {
        Self {
            inner: List::default(),
        }
    }
}
