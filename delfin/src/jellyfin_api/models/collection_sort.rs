/// Ordering for a complete collection, applied by Jellyfin before pagination.
/// The variant order matches the collection's sort dropdown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CollectionSort {
    #[default]
    NameAscending,
    NameDescending,
    DateAddedDescending,
    DateAddedAscending,
}

impl CollectionSort {
    pub fn from_selected(selected: u32) -> Option<Self> {
        match selected {
            0 => Some(Self::NameAscending),
            1 => Some(Self::NameDescending),
            2 => Some(Self::DateAddedDescending),
            3 => Some(Self::DateAddedAscending),
            _ => None,
        }
    }

    pub fn sort_by(self) -> &'static str {
        match self {
            Self::NameAscending | Self::NameDescending => "SortName,ProductionYear",
            Self::DateAddedDescending | Self::DateAddedAscending => {
                "DateCreated,SortName,ProductionYear"
            }
        }
    }

    pub fn sort_order(self) -> &'static str {
        match self {
            Self::NameAscending | Self::DateAddedAscending => "Ascending",
            Self::NameDescending | Self::DateAddedDescending => "Descending",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CollectionSort;

    #[test]
    fn dropdown_choices_map_to_server_sort_parameters() {
        let expected = [
            ("SortName,ProductionYear", "Ascending"),
            ("SortName,ProductionYear", "Descending"),
            ("DateCreated,SortName,ProductionYear", "Descending"),
            ("DateCreated,SortName,ProductionYear", "Ascending"),
        ];
        for (index, (sort_by, sort_order)) in expected.into_iter().enumerate() {
            let sort = CollectionSort::from_selected(index as u32).unwrap();
            assert_eq!(sort.sort_by(), sort_by);
            assert_eq!(sort.sort_order(), sort_order);
        }
    }

    #[test]
    fn invalid_selection_is_ignored() {
        assert_eq!(CollectionSort::from_selected(4), None);
        assert_eq!(CollectionSort::from_selected(u32::MAX), None);
    }

    #[test]
    fn default_preserves_alphabetical_order() {
        assert_eq!(CollectionSort::default(), CollectionSort::NameAscending);
        assert_eq!(
            CollectionSort::from_selected(0),
            Some(CollectionSort::default())
        );
    }

    #[test]
    fn date_added_orders_use_name_and_year_as_secondary_sort_keys() {
        for selected in 2..4 {
            assert_eq!(
                CollectionSort::from_selected(selected).unwrap().sort_by(),
                "DateCreated,SortName,ProductionYear"
            );
        }
    }
}
