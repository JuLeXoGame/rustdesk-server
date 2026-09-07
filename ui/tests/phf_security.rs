#[test]
fn generated_map_matches_runtime_lookup() {
    let entries = ["", "localhost", "écran", "第三", "relaisdesk.fr"];
    let first = phf_generator::generate_hash(&entries);
    let second = phf_generator::generate_hash(&entries);
    assert_eq!(first.key, second.key);
    assert_eq!(first.disps, second.disps);
    assert_eq!(first.map, second.map);
    for (index, entry) in entries.iter().enumerate() {
        let hash = phf_shared::hash(entry, &first.key);
        let slot = phf_shared::get_index(&hash, &first.disps, entries.len());
        assert_eq!(first.map[slot as usize], index);
    }
    assert!(phf_generator::generate_hash::<&str>(&[]).map.is_empty());
}
