pub struct RegionCode {}

#[allow(unused)]
impl RegionCode {
    pub fn ad() -> &'static str {
        "AD"
    }

    pub fn ae() -> &'static str {
        "AE"
    }

    pub fn am() -> &'static str {
        "AM"
    }

    pub fn ao() -> &'static str {
        "AO"
    }

    pub fn aq() -> &'static str {
        "AQ"
    }

    pub fn ar() -> &'static str {
        "AR"
    }

    pub fn au() -> &'static str {
        "AU"
    }

    pub fn bb() -> &'static str {
        "BB"
    }

    pub fn br() -> &'static str {
        "BR"
    }

    pub fn bs() -> &'static str {
        "BS"
    }

    pub fn by() -> &'static str {
        "BY"
    }

    pub fn ca() -> &'static str {
        "CA"
    }

    pub fn ch() -> &'static str {
        "CH"
    }

    pub fn cl() -> &'static str {
        "CL"
    }

    pub fn cn() -> &'static str {
        "CN"
    }

    pub fn co() -> &'static str {
        "CO"
    }

    pub fn cs() -> &'static str {
        "CS"
    }

    pub fn cx() -> &'static str {
        "CX"
    }

    pub fn de() -> &'static str {
        "DE"
    }

    pub fn fr() -> &'static str {
        "FR"
    }

    pub fn gb() -> &'static str {
        "GB"
    }

    pub fn hu() -> &'static str {
        "HU"
    }

    pub fn it() -> &'static str {
        "IT"
    }

    pub fn jp() -> &'static str {
        "JP"
    }

    pub fn kr() -> &'static str {
        "KR"
    }

    pub fn mx() -> &'static str {
        "MX"
    }

    pub fn nz() -> &'static str {
        "NZ"
    }

    pub fn pl() -> &'static str {
        "PL"
    }

    pub fn re() -> &'static str {
        "RE"
    }

    pub fn ru() -> &'static str {
        "RU"
    }

    pub fn se() -> &'static str {
        "SE"
    }

    pub fn sg() -> &'static str {
        "SG"
    }

    pub fn un001() -> &'static str {
        "001"
    }

    pub fn us() -> &'static str {
        "US"
    }

    pub fn uz() -> &'static str {
        "UZ"
    }

    pub fn yt() -> &'static str {
        "YT"
    }

    pub fn zw() -> &'static str {
        "ZW"
    }

    /// s a region code string representing the "unknown" region.
    pub fn get_unknown() -> &'static str {
        Self::zz()
    }

    pub fn zz() -> &'static str {
        "ZZ"
    }
}
