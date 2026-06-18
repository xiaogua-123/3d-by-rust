//! 空间查询分类过滤器
//!
//! 定义 `CategoryFilter`：`All` / `Include(...)` / `Exclude(...)`。

use crate::td::spatial::entry::EntityCategory;

#[derive(Clone, Copy, Debug)]
pub enum CategoryFilter {
    #[allow(dead_code)]
    All,
    Include(EntityCategory),
    #[allow(dead_code)]
    Exclude(EntityCategory),
}

impl CategoryFilter {
    #[inline]
    pub fn matches(&self, category: &EntityCategory) -> bool {
        match self {
            CategoryFilter::All => true,
            CategoryFilter::Include(c) => c == category,
            CategoryFilter::Exclude(c) => c != category,
        }
    }

    pub fn monster_only() -> Self {
        CategoryFilter::Include(EntityCategory::Monster)
    }

    #[allow(dead_code)]
    pub fn tower_only() -> Self {
        CategoryFilter::Include(EntityCategory::Tower)
    }

    #[allow(dead_code)]
    pub fn exclude(category: EntityCategory) -> Self {
        CategoryFilter::Exclude(category)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_include_matches() {
        let filter = CategoryFilter::Include(EntityCategory::Monster);
        assert!(filter.matches(&EntityCategory::Monster));
        assert!(!filter.matches(&EntityCategory::Tower));
    }

    #[test]
    fn test_all_matches() {
        let filter = CategoryFilter::All;
        assert!(filter.matches(&EntityCategory::Monster));
        assert!(filter.matches(&EntityCategory::Tower));
    }

    #[test]
    fn test_monster_only() {
        let filter = CategoryFilter::monster_only();
        assert!(filter.matches(&EntityCategory::Monster));
        assert!(!filter.matches(&EntityCategory::Tower));
    }
}
