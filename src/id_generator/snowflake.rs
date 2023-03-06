lazy_static::lazy_static!{
    static ref SNOW_FLAKE_ID_GENERATOR: wd_sonyflake::SonyFlakeEntity = wd_sonyflake::SonyFlakeEntity::new_default();
}

pub fn snowflake_id()->i64{
    SNOW_FLAKE_ID_GENERATOR.get_id()
}