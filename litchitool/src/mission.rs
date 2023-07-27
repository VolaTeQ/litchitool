use std::hash::{Hash, Hasher};

use bytes::{BufMut, Bytes, BytesMut};
use num_enum::TryFromPrimitive;

use crate::error::LitchiError;

/// Cardinal coordinates (latitude, longitude)
#[derive(Debug, Clone, PartialEq)]
pub struct Coordinate(pub f64, pub f64);

#[derive(Debug, Clone, Copy)]
pub enum Action {
    StayFor(f32),
    TakePhoto,
    StartRecording,
    StopRecording,
    RotateAircraft(i32),
    TiltCamera(i32),
}

#[derive(Debug, Clone)]
pub enum PhotoInterval {
    /// Time in seconds
    Time(f32),
    /// Distance in meters
    Distance(f32),
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum HeadingMode {
    Auto,
    Initial,
    Manual,
    Custom,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum FinishAction {
    None,
    Rth,
    Land,
    BackToFirst,
    Reverse,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum PathMode {
    StraightLines,
    CurvedTurns,
}

#[derive(Debug, Clone, Copy, TryFromPrimitive)]
#[repr(i32)]
pub enum GimbalPitchMode {
    Disabled,
    FocusPOI,
    Interpolate,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, TryFromPrimitive)]
#[repr(i16)]
pub enum AltitudeMode {
    Absolute,
    AboveGround,
}

#[derive(Debug, Clone)]
pub struct Waypoint {
    pub coordinates: Coordinate,
    pub altitude: f32,
    /// Heading of the waypoint, must be between -180 and 180 TODO: validate
    pub heading: f32,
    pub curve_size: f32,
    pub rotation_dir: i32,
    pub gimbal_mode: GimbalPitchMode,
    pub gimbal_pitch_angle: i32,
    pub altitude_mode: AltitudeMode,
    pub speed: f32,
    pub poi_index: Option<usize>,
    pub actions: Vec<Action>,
    pub photo_interval: Option<PhotoInterval>,
    pub turn_mode: i32, // TODO: enum, what is this??
    pub stay_time: i16,
    pub max_reach_time: i16,
    pub repeat_actions: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct POI {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f32,
    pub altitude_mode: AltitudeMode,
}

#[derive(Debug, Clone)]
pub struct MissionConfig {
    pub heading_mode: HeadingMode,
    pub finish_action: FinishAction,
    pub path_mode: PathMode,
    pub cruising_speed: f32,
    pub rc_speed: f32,
    pub n_repeat: i32,
    pub version: i16,
    pub photo_interval: Option<PhotoInterval>,
}

#[derive(Debug, Default, Clone)]
pub struct LitchiMission {
    waypoints: Vec<Waypoint>,
    pois: Vec<POI>,
    config: MissionConfig,
}

impl LitchiMission {
    pub fn new(
        waypoints: Vec<Waypoint>,
        pois: Vec<POI>,
        config: MissionConfig,
    ) -> Result<Self, LitchiError> {
        let new = Self {
            waypoints,
            pois,
            config,
        };

        if new.validate() {
            Ok(new)
        } else {
            Err(LitchiError::InvalidMission)
        }
    }

    fn validate(&self) -> bool {
        // TODO: Check coordinates, heights, speeds, angles, etc.

        self.waypoints.iter().all(|waypoint| {
            waypoint
                .poi_index
                .map_or(true, |index| index < self.pois.len())
        })
    }

    pub fn pois(&self) -> &Vec<POI> {
        &self.pois
    }

    pub fn waypoints(&self) -> &Vec<Waypoint> {
        &self.waypoints
    }

    pub fn config(&self) -> &MissionConfig {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut MissionConfig {
        &mut self.config
    }

    /// Converts the mission to the litchi binary mission format
    pub fn to_binary(&self) -> Bytes {
        // TODO: Calculate final size and use BytesMut::with_capacity(capacity);
        let mut buf = BytesMut::new();

        // Litchi file format signature
        buf.put_i32(1818454125);
        // Heading mode
        buf.put_i32(self.config.heading_mode as i32);
        // Finish action
        buf.put_i32(self.config.finish_action as i32);
        // Path mode
        buf.put_i32(self.config.path_mode as i32);
        // Cruising speed (must be clamped at -15 and 15 for some reason)
        buf.put_f32(self.config.cruising_speed.clamp(-15., 15.));
        // Rc speed (clamed at 2..15)
        buf.put_f32(self.config.rc_speed.clamp(2., 15.));
        // Number of repetitions
        buf.put_i32(self.config.n_repeat);
        // Set version (hardcoded at 11)
        buf.put_i16(11);
        // (padding)
        buf.put_slice(&[0u8; 10]);

        // Number of waypoints
        buf.put_i32(
            self.waypoints
                .len()
                .try_into()
                .expect("Number of waypoints must fit in i32"),
        );

        // Waypoint information
        for waypoint in &self.waypoints {
            buf.put_f32(waypoint.altitude);
            buf.put_i32(waypoint.turn_mode);
            buf.put_f32(waypoint.heading);
            buf.put_f32(waypoint.speed);
            buf.put_i16(waypoint.stay_time);
            buf.put_i16(waypoint.max_reach_time);
            buf.put_f64(waypoint.coordinates.0);
            buf.put_f64(waypoint.coordinates.1);
            buf.put_f32(waypoint.curve_size);
            buf.put_i32(waypoint.gimbal_mode as i32);
            buf.put_i32(waypoint.gimbal_pitch_angle);
            buf.put_i32(
                waypoint
                    .actions
                    .len()
                    .try_into()
                    .expect("Number of waypoint actions must fit in i32"),
            );
            buf.put_i32(waypoint.repeat_actions);

            for action in &waypoint.actions {
                let (action_n, param) = action.idx_and_param();
                buf.put_i32(action_n);
                buf.put_i32(param);
            }
        }

        // Number of POI's
        buf.put_i32(
            self.pois
                .len()
                .try_into()
                .expect("Number of POI's must fit in i32"),
        );

        // POI positions
        for poi in &self.pois {
            buf.put_f64(poi.latitude);
            buf.put_f64(poi.longitude);
            buf.put_f32(poi.altitude);
        }

        // Set waypoint altitude and POI info
        for waypoint in &self.waypoints {
            buf.put_i16(waypoint.altitude_mode as i16);
            buf.put_f32(waypoint.altitude as f32);
            buf.put_i32(waypoint.poi_index.map_or(-1, |index| {
                index.try_into().expect("POI index must fit into i32")
            }));
        }

        // Set POI altitude info
        for poi in &self.pois {
            buf.put_i16(poi.altitude_mode as i16);
            buf.put_f32(poi.altitude);
        }

        // Magic numbers

        buf.put_i32(8);
        buf.put_i32(8);
        buf.put_i32(0);

        // Set global photo intervals

        let mut set_interval = |interval: Option<&PhotoInterval>| {
            if let Some(interval) = interval {
                match interval {
                    PhotoInterval::Time(time) => {
                        buf.put_f32(*time);
                        buf.put_f32(-1.);
                    }
                    PhotoInterval::Distance(distance) => {
                        buf.put_f32(-1.);
                        buf.put_f32(*distance);
                    }
                }
            } else {
                buf.put_f32(-1.);
                buf.put_f32(-1.);
            }
        };

        set_interval(self.config.photo_interval.as_ref());

        for waypoint in &self.waypoints {
            set_interval(waypoint.photo_interval.as_ref());
        }

        buf.freeze()
    }
}

impl Default for MissionConfig {
    fn default() -> Self {
        Self {
            heading_mode: HeadingMode::Manual,
            finish_action: FinishAction::Rth,
            path_mode: PathMode::StraightLines,
            cruising_speed: 8.,
            rc_speed: 14.,
            n_repeat: 1,
            version: 11,
            photo_interval: None,
        }
    }
}

impl Hash for POI {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.latitude.to_bits().hash(state);
        self.longitude.to_bits().hash(state);
        self.altitude.to_bits().hash(state);
        self.altitude_mode.hash(state);
    }
}

impl Action {
    fn idx_and_param(&self) -> (i32, i32) {
        match self {
            Self::StayFor(stay) => (0, (stay * 1000.) as i32),
            Self::TakePhoto => (1, 0),
            Self::StartRecording => (2, 0),
            Self::StopRecording => (3, 0),
            Self::RotateAircraft(rotation) => (4, *rotation),
            Self::TiltCamera(tilt) => (5, *tilt),
        }
    }
}
