enum Identification{
    NameAndBirth{first_name: String, last_name: String, birth_date: Date},
    Passport{number: String, expiration_date: Date},
    DriverLicense{number: String, expiration_date: Date},
    SocialSecurityNumber{number: String},
    PassportAndDriverLicense{passport_number: String, driver_license_number: String, expiration_date: Date},
    PassportAndSocialSecurityNumber{passport_number: String, social_security_number: String},
    DriverLicenseAndSocialSecurityNumber{driver_license_number: String, social_security_number: String},
    PassportAndDriverLicenseAndSocialSecurityNumber{passport_number: String, driver_license_number: String, social_security_number: String},}
}

enum IdentificationHash{
    NameAndBirth(String),
    Passport(String),
    DriverLicense(String),
    SocialSecurityNumber(String),
    PassportAndDriverLicense(String),
    PassportAndSocialSecurityNumber(String),
    DriverLicenseAndSocialSecurityNumber(String),
    PassportAndDriverLicenseAndSocialSecurityNumber(String),
}
