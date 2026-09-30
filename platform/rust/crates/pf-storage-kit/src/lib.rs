use pf_errors::{PlatformError, PlatformResult};
use std::collections::HashMap;

pub const DEFAULT_PAGE_SIZE: usize = 4096;

/// Fixed-size storage page buffer.
#[derive(Clone)]
pub struct Page {
    pub page_id: u64,
    pub data: [u8; DEFAULT_PAGE_SIZE],
    pub is_dirty: bool,
    pub pin_count: u32,
}

impl Page {
    pub fn new(page_id: u64) -> Self {
        Self {
            page_id,
            data: [0u8; DEFAULT_PAGE_SIZE],
            is_dirty: false,
            pin_count: 0,
        }
    }
}

/// Buffer Pool Manager managing in-memory page frames.
pub struct BufferPool {
    pool_size: usize,
    pages: HashMap<u64, Page>,
}

impl BufferPool {
    pub fn new(pool_size: usize) -> Self {
        Self {
            pool_size,
            pages: HashMap::with_capacity(pool_size),
        }
    }

    pub fn fetch_page(&mut self, page_id: u64) -> PlatformResult<&mut Page> {
        if !self.pages.contains_key(&page_id) {
            if self.pages.len() >= self.pool_size {
                // Find unpinned page to evict
                let evict_id = self
                    .pages
                    .iter()
                    .find(|(_, p)| p.pin_count == 0)
                    .map(|(&id, _)| id)
                    .ok_or_else(|| {
                        PlatformError::Storage("Buffer pool full; all pages pinned".into())
                    })?;
                self.pages.remove(&evict_id);
            }
            self.pages.insert(page_id, Page::new(page_id));
        }

        let page = self.pages.get_mut(&page_id).unwrap();
        page.pin_count += 1;
        Ok(page)
    }

    pub fn unpin_page(&mut self, page_id: u64, is_dirty: bool) -> PlatformResult<()> {
        if let Some(page) = self.pages.get_mut(&page_id) {
            if page.pin_count > 0 {
                page.pin_count -= 1;
            }
            if is_dirty {
                page.is_dirty = true;
            }
            Ok(())
        } else {
            Err(PlatformError::NotFound(format!(
                "Page {} not in pool",
                page_id
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_pool() {
        let mut pool = BufferPool::new(2);
        let p1 = pool.fetch_page(1).unwrap();
        p1.data[0] = 42;
        pool.unpin_page(1, true).unwrap();

        let p2 = pool.fetch_page(2).unwrap();
        p2.data[0] = 99;
        pool.unpin_page(2, false).unwrap();

        let p1_again = pool.fetch_page(1).unwrap();
        assert_eq!(p1_again.data[0], 42);
    }
}
