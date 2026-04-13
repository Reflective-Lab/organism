/// Swedish Personnummer (Personal Identity Number) utilities
///
/// Format: YYYYMMDD-XXXX (12 digits) or YYMMDD-XXXX (10 digits)
///
/// Structure of last 4 digits (XXXX):
/// - First 3 digits: Birth number (unique for people born same day)
/// - Last digit: Luhn checksum
///
/// Gender: Second-to-last digit is odd for male, even for female

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
}

#[derive(Debug, Clone)]
pub struct Personnummer {
    /// The normalized 10-digit format (YYMMDDXXXX, no hyphen)
    digits: String,
    /// Full year (e.g., 1964)
    year: u16,
}

#[derive(Debug, thiserror::Error)]
pub enum PersonnummerError {
    #[error("Invalid format: expected YYYYMMDD-XXXX or YYMMDD-XXXX")]
    InvalidFormat,
    #[error("Invalid date: {0}")]
    InvalidDate(String),
    #[error("Invalid checksum: expected {expected}, got {actual}")]
    InvalidChecksum { expected: u8, actual: u8 },
}

impl Personnummer {
    /// Parse and validate a Swedish personnummer
    pub fn parse(input: &str) -> Result<Self, PersonnummerError> {
        // Remove any hyphens or plus signs
        let cleaned: String = input.chars().filter(|c| c.is_ascii_digit()).collect();

        let (year, digits) = match cleaned.len() {
            12 => {
                // YYYYMMDDXXXX
                let year: u16 = cleaned[0..4]
                    .parse()
                    .map_err(|_| PersonnummerError::InvalidFormat)?;
                let digits = format!("{}{}", &cleaned[2..8], &cleaned[8..12]);
                (year, digits)
            }
            10 => {
                // YYMMDDXXXX - need to determine century
                let yy: u16 = cleaned[0..2]
                    .parse()
                    .map_err(|_| PersonnummerError::InvalidFormat)?;
                // Use + for people over 100, otherwise assume 1900s or 2000s
                let year = if input.contains('+') {
                    // Person is over 100 years old
                    if yy > 24 { 1800 + yy } else { 1900 + yy }
                } else if yy > 24 {
                    1900 + yy
                } else {
                    2000 + yy
                };
                (year, cleaned)
            }
            _ => return Err(PersonnummerError::InvalidFormat),
        };

        let pnr = Self { digits, year };

        // Validate checksum
        pnr.validate_checksum()?;

        // Validate date
        pnr.validate_date()?;

        Ok(pnr)
    }

    /// Calculate the Luhn checksum for the first 9 digits
    fn calculate_checksum(digits: &str) -> u8 {
        let multipliers = [2, 1, 2, 1, 2, 1, 2, 1, 2];

        let sum: u32 = digits[..9]
            .chars()
            .zip(multipliers.iter())
            .map(|(c, &mult)| {
                let digit = c.to_digit(10).unwrap();
                let product = digit * mult;
                // If product >= 10, sum its digits (e.g., 12 -> 1+2 = 3)
                if product >= 10 {
                    product - 9
                } else {
                    product
                }
            })
            .sum();

        ((10 - (sum % 10)) % 10) as u8
    }

    fn validate_checksum(&self) -> Result<(), PersonnummerError> {
        let expected = Self::calculate_checksum(&self.digits);
        let actual = self.digits.chars().last().unwrap().to_digit(10).unwrap() as u8;

        if expected != actual {
            return Err(PersonnummerError::InvalidChecksum { expected, actual });
        }
        Ok(())
    }

    fn validate_date(&self) -> Result<(), PersonnummerError> {
        let month: u8 = self.digits[2..4]
            .parse()
            .map_err(|_| PersonnummerError::InvalidDate("invalid month".into()))?;
        let day: u8 = self.digits[4..6]
            .parse()
            .map_err(|_| PersonnummerError::InvalidDate("invalid day".into()))?;

        // Handle coordination numbers (day + 60 for non-residents)
        let actual_day = if day > 60 { day - 60 } else { day };

        if !(1..=12).contains(&month) {
            return Err(PersonnummerError::InvalidDate(format!(
                "month {} out of range",
                month
            )));
        }

        let max_day = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if self.is_leap_year() {
                    29
                } else {
                    28
                }
            }
            _ => unreachable!(),
        };

        if actual_day < 1 || actual_day > max_day {
            return Err(PersonnummerError::InvalidDate(format!(
                "day {} out of range for month {}",
                actual_day, month
            )));
        }

        Ok(())
    }

    fn is_leap_year(&self) -> bool {
        (self.year % 4 == 0 && self.year % 100 != 0) || (self.year % 400 == 0)
    }

    /// Get the gender indicated by this personnummer
    pub fn gender(&self) -> Gender {
        let second_last = self.digits.chars().nth(8).unwrap().to_digit(10).unwrap();
        if second_last % 2 == 1 {
            Gender::Male
        } else {
            Gender::Female
        }
    }

    /// Get the birth date as (year, month, day)
    pub fn birth_date(&self) -> (u16, u8, u8) {
        let month: u8 = self.digits[2..4].parse().unwrap();
        let day: u8 = self.digits[4..6].parse().unwrap();
        // Handle coordination numbers
        let actual_day = if day > 60 { day - 60 } else { day };
        (self.year, month, actual_day)
    }

    /// Format as 12-digit with hyphen (YYYYMMDD-XXXX)
    pub fn format_long(&self) -> String {
        format!(
            "{}{}-{}",
            self.year,
            &self.digits[2..6],
            &self.digits[6..10]
        )
    }

    /// Format as 10-digit with hyphen (YYMMDD-XXXX)
    pub fn format_short(&self) -> String {
        format!("{}-{}", &self.digits[0..6], &self.digits[6..10])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_personnummer() {
        let pnr = Personnummer::parse("19640121-1013").unwrap();
        assert_eq!(pnr.gender(), Gender::Male);
        assert_eq!(pnr.birth_date(), (1964, 1, 21));
        assert_eq!(pnr.format_long(), "19640121-1013");
        assert_eq!(pnr.format_short(), "640121-1013");
    }

    #[test]
    fn test_short_format() {
        let pnr = Personnummer::parse("640121-1013").unwrap();
        assert_eq!(pnr.gender(), Gender::Male);
    }

    #[test]
    fn test_invalid_checksum() {
        let result = Personnummer::parse("19640121-1014");
        assert!(matches!(
            result,
            Err(PersonnummerError::InvalidChecksum { .. })
        ));
    }

    #[test]
    fn test_female() {
        // Even second-to-last digit = female
        // 19850101-2382: second-to-last is 8 (even) = female
        let pnr = Personnummer::parse("19850101-2382").unwrap();
        assert_eq!(pnr.gender(), Gender::Female);
    }
}
