use std::io::Read;

pub use csv;
use csv::Reader;
use tracing::Level;

use crate::{
    error::LitchiError,
    mission::{
        Action, AltitudeMode, Coordinate, GimbalPitchMode, LitchiMission, MissionConfig,
        PhotoInterval, Waypoint, POI,
    },
};

macro_rules! parse_chunk {
    ($record:expr => $($name:ident ($type:ty) $idx:expr),+) => {
        $(
            let $name: $type = $record.get($idx).ok_or(LitchiError::CsvMissingField($idx))?.parse()?;
        )+
    }
}

pub fn read_from_csv<R: Read>(mut reader: Reader<R>) -> Result<LitchiMission, LitchiError> {
    let mut waypoints: Vec<Waypoint> = vec![];
    let mut pois: Vec<POI> = vec![];

    for (record_index, record) in reader.records().enumerate() {
        let row_span = tracing::span!(Level::TRACE, "Parsing row of CSV", row = record_index);
        let _enter = row_span.enter();
        let record = record?;
        if record.len() != 46 {
            Err(LitchiError::IncorrectRecordLength(record.len(), 46))?;
        }

        const ACTIONS_OFFSET: usize = 8;
        const ACTIONS_COUNT: usize = 15;
        const ACTIONS_END: usize = ACTIONS_OFFSET + ACTIONS_COUNT * 2;

        parse_chunk!(record =>
            latitude             (f64) 0,
            longitude            (f64) 1,
            altitude             (f32) 2,
            heading              (f32) 3,
            curve_size           (f32) 4,
            rotation_dir         (i32) 5,
            gimbal_mode          (i32) 6,
            gimbal_pitch_angle   (i32) 7,
            altitude_mode       (i16) ACTIONS_END,
            speed               (f32) ACTIONS_END + 1,
            poi_latitude        (f64) ACTIONS_END + 2,
            poi_longitude       (f64) ACTIONS_END + 3,
            poi_altitude        (f32) ACTIONS_END + 4,
            poi_altitude_mode   (i16) ACTIONS_END + 5,
            photo_time_interval (f32) ACTIONS_END + 6,
            photo_distance_interval (f32) ACTIONS_END + 7
        );

        let mut heading = heading;

        let gimbal_mode = GimbalPitchMode::try_from(gimbal_mode)
            .map_err(|err| LitchiError::TryFromPrimitiveError(err.number.to_string()))?;
        let altitude_mode = AltitudeMode::try_from(altitude_mode)
            .map_err(|err| LitchiError::TryFromPrimitiveError(err.number.to_string()))?;
        let photo_time_interval = Some(photo_time_interval).filter(|interval| *interval > 0.);
        let photo_distance_interval =
            Some(photo_distance_interval).filter(|interval| *interval > 0.);

        let actions = (0..ACTIONS_COUNT)
            .map(|action_i| -> Result<Option<Action>, LitchiError> {
                parse_chunk!(record =>
                    action_type (i32) ACTIONS_OFFSET + action_i * 2,
                    action_param (i32) ACTIONS_OFFSET + 1 + action_i * 2
                );

                Ok(match action_type {
                    -1 => None,
                    0 => Some(Action::StayFor(action_param as f32 / 1000.)),
                    1 => Some(Action::TakePhoto),
                    2 => Some(Action::StartRecording),
                    3 => Some(Action::StopRecording),
                    4 => Some(Action::RotateAircraft(action_param)),
                    5 => Some(Action::TiltCamera(action_param)),
                    n => Err(LitchiError::InvalidActionType(n))?,
                })
            })
            .filter_map(|res| match res {
                Ok(Some(action)) => Some(Ok(action)),
                Ok(None) => None,
                Err(error) => Some(Err(error)),
            })
            .collect::<Result<Vec<_>, _>>()?;

        let poi = {
            let poi_altitude_mode = AltitudeMode::try_from(poi_altitude_mode)
                .map_err(|err| LitchiError::TryFromPrimitiveError(err.number.to_string()))?;

            if poi_longitude != 0. && poi_latitude != 0. && poi_altitude != 0. {
                Some(POI {
                    coordinate: Coordinate(poi_latitude, poi_longitude),
                    altitude: poi_altitude,
                    altitude_mode: poi_altitude_mode,
                })
            } else {
                None
            }
        };

        let coordinates = Coordinate(latitude, longitude);

        if let Some(poi) = &poi {
            heading = coordinates.heading_towards(&poi.coordinate) as f32;
        }

        let poi_index = poi.map(|poi| {
            pois.iter()
                .position(|search_poi| &poi == search_poi)
                .unwrap_or_else(|| {
                    pois.push(poi);
                    pois.len() - 2
                })
        });

        waypoints.push(Waypoint {
            coordinate: coordinates,
            altitude,
            altitude_mode,
            heading,
            curve_size,
            gimbal_mode,
            gimbal_pitch_angle,
            speed,
            poi_index,
            actions,
            turn_mode: 0,
            photo_interval: photo_time_interval
                .map(PhotoInterval::Time)
                .or_else(|| photo_distance_interval.map(PhotoInterval::Distance)),
            repeat_actions: 1,
            rotation_dir,
            stay_time: 3,
            max_reach_time: 0,
        })
    }

    LitchiMission::new(waypoints, pois, MissionConfig::default())
}
