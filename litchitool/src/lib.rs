pub mod csv_format;
pub mod error;
pub mod mission;

#[cfg(test)]
mod tests {
    use crate::csv_format;

    #[test]
    fn test_convert_mission() {
        const TEST_MISSION_CSV: &[u8] = include_bytes!("../test/litchi_mission.csv");

        let mission = csv_format::read_from_csv(csv::Reader::from_reader(TEST_MISSION_CSV))
            .expect("Could not parse test mission from csv");

        let binary = mission.to_binary();

        insta::assert_debug_snapshot!(binary);
    }

    // TODO: More tests
}
