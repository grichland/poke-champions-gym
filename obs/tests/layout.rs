//! The generated layout: totals, no overlap, every element owned by exactly one feature, constants
//! agree with the CSV.

use obs::layout::*;
use obs::Block;

fn len(b: Block) -> usize {
    match b {
        Block::Row => ROW_LEN,
        Block::RowIds => ROWIDS_LEN,
        Block::Field => FIELD_LEN,
        Block::Last => LAST_LEN,
        Block::LastIds => LASTIDS_LEN,
        Block::Mask => MASK_LEN,
        Block::Table => TABLE_LEN,
    }
}

#[test]
fn totals_match_the_csv() {
    assert_eq!((ROW_LEN, ROWIDS_LEN, FIELD_LEN, LAST_LEN, LASTIDS_LEN, MASK_LEN), (300, 10, 66, 17, 4, 11));
    assert_eq!(FEATURES.len(), 149);
    assert_eq!(MOVE_BASE + 4 * MOVE_STRIDE, F9_1_1);
}

#[test]
fn every_element_has_exactly_one_feature() {
    for b in [Block::Row, Block::RowIds, Block::Field, Block::Last, Block::LastIds, Block::Mask] {
        let mut owner: Vec<Option<&str>> = vec![None; len(b)];
        for f in FEATURES.iter().filter(|f| f.block == b) {
            let starts: Vec<usize> = if f.slot { (0..4).map(|s| MOVE_BASE + MOVE_STRIDE * s + f.offset).collect() } else { vec![f.offset] };
            // a move-slot float block repeats by stride; an id list of 4 is contiguous
            let span = if f.slot { f.size } else { f.size * f.repeats };
            for s in starts {
                for i in s..s + span {
                    assert!(i < owner.len(), "{} past the end of {:?}", f.id, b);
                    assert!(owner[i].is_none(), "{} overlaps {:?} at {:?}[{}]", f.id, owner[i], b, i);
                    owner[i] = Some(f.id);
                }
            }
        }
        let free: Vec<usize> = owner.iter().enumerate().filter(|(_, o)| o.is_none()).map(|(i, _)| i).collect();
        assert!(free.is_empty(), "{:?}: unassigned {:?}", b, free);
    }
}

#[test]
fn ids_are_unique_and_csv_rows_parse() {
    let mut ids: Vec<&str> = FEATURES.iter().map(|f| f.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), FEATURES.len());
    let csv = include_str!("../features_proposed.csv");
    let lines: Vec<&str> = csv.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), FEATURES.len());
    for (l, f) in lines.iter().zip(FEATURES.iter()) {
        assert!(l.starts_with(&format!("{},{},", f.id, f.name)), "{} vs {}", l, f.id);
    }
}

#[test]
fn ranges_are_ordered() {
    for f in FEATURES.iter() {
        assert!(f.lo < f.hi, "{}", f.id);
    }
}
