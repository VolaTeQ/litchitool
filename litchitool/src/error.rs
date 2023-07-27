use thiserror::Error;

#[derive(Debug, Error)]
pub enum LitchiError {
	#[error("Error while reading csv: {0:?}")]
	CsvError(#[from] csv::Error),
	#[error("Incorrect length of csv record, got {0} but expected {1}")]
	IncorrectRecordLength(usize, usize),
	#[error("Field #{0} of the CSV is missing, this error should never appear")]
	CsvMissingField(usize),
	#[error("Failed to parse float: {0:?}")]
	ParseFloatError(#[from] std::num::ParseFloatError),
	#[error("Failed to parse Integer: {0:?}")]
	ParseIntError(#[from] std::num::ParseIntError),
	#[error("Invalid action type {0}")]
	InvalidActionType(i32),
	#[error("Could not convert number to enum value: {0:?}")]
	TryFromPrimitiveError(String),
	#[error("Invalid mission")]
	InvalidMission, // TODO: Reason
}
