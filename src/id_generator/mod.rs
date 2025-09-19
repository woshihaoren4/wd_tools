#[cfg(feature = "snowflake")]
mod snowflake;

#[cfg(feature = "snowflake")]
pub use wd_sonyflake::SonyFlakeEntity as Snowflake;

#[cfg(feature = "snowflake")]
pub use snowflake::snowflake_id;

#[cfg(feature = "uid")]
pub mod uuid;

#[cfg(feature = "random")]
pub mod rand;
