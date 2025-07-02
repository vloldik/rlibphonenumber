pub struct RegionCode {
}

impl RegionCode {
    /// Returns a region code string representing the "unknown" region.
    pub fn get_unknown() -> &'static str {
        return Self::zz();
    }

    pub fn zz() -> &'static str {
        return "ZZ";    
    }
}