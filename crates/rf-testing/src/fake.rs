//! Comprehensive fake data generator for testing
//!
//! This module provides a comprehensive fake data generation API similar to Laravel's Faker,
//! built on top of the `fake` crate with additional convenience methods.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use rand::Rng;

/// Comprehensive fake data generator
pub struct Fake;

impl Fake {
    // ====== Names ======

    /// Generate a full name
    ///
    /// # Example
    /// ```
    /// use rf_testing::Fake;
    /// let name = Fake::name();
    /// assert!(!name.is_empty());
    /// ```
    pub fn name() -> String {
        use fake::faker::name::en::Name;
        use fake::Fake as FakeTrait;
        Name().fake()
    }

    /// Generate a first name
    pub fn first_name() -> String {
        use fake::faker::name::en::FirstName;
        use fake::Fake as FakeTrait;
        FirstName().fake()
    }

    /// Generate a last name
    pub fn last_name() -> String {
        use fake::faker::name::en::LastName;
        use fake::Fake as FakeTrait;
        LastName().fake()
    }

    /// Generate a username
    pub fn username() -> String {
        use fake::faker::internet::en::Username;
        use fake::Fake as FakeTrait;
        Username().fake()
    }

    /// Generate a name with title (e.g., "Dr. John Smith")
    pub fn name_with_title() -> String {
        use fake::faker::name::en::NameWithTitle;
        use fake::Fake as FakeTrait;
        NameWithTitle().fake()
    }

    // ====== Contact ======

    /// Generate an email address
    pub fn email() -> String {
        use fake::faker::internet::en::SafeEmail;
        use fake::Fake as FakeTrait;
        SafeEmail().fake()
    }

    /// Generate a free email address (gmail, yahoo, etc.)
    pub fn free_email() -> String {
        use fake::faker::internet::en::FreeEmail;
        use fake::Fake as FakeTrait;
        FreeEmail().fake()
    }

    /// Generate a phone number
    pub fn phone() -> String {
        use fake::faker::phone_number::en::PhoneNumber;
        use fake::Fake as FakeTrait;
        PhoneNumber().fake()
    }

    /// Generate an international phone number with country code
    pub fn phone_international() -> String {
        let country_code = Self::number(1, 999);
        let area = Self::number(100, 999);
        let exchange = Self::number(100, 999);
        let number = Self::number(1000, 9999);
        format!("+{} ({}) {}-{}", country_code, area, exchange, number)
    }

    /// Generate a cell phone number
    pub fn cell_phone() -> String {
        use fake::faker::phone_number::en::CellNumber;
        use fake::Fake as FakeTrait;
        CellNumber().fake()
    }

    // ====== Address ======

    /// Generate a full address
    pub fn address() -> String {
        format!(
            "{}, {}, {} {}",
            Self::street(),
            Self::city(),
            Self::state_abbr(),
            Self::zip()
        )
    }

    /// Generate a street address
    pub fn street() -> String {
        use fake::faker::address::en::StreetName;
        use fake::Fake as FakeTrait;
        let street_name: String = StreetName().fake();
        let number = Self::number(1, 9999);
        format!("{} {}", number, street_name)
    }

    /// Generate a city name
    pub fn city() -> String {
        use fake::faker::address::en::CityName;
        use fake::Fake as FakeTrait;
        CityName().fake()
    }

    /// Generate a state name
    pub fn state() -> String {
        use fake::faker::address::en::StateName;
        use fake::Fake as FakeTrait;
        StateName().fake()
    }

    /// Generate a state abbreviation
    pub fn state_abbr() -> String {
        use fake::faker::address::en::StateAbbr;
        use fake::Fake as FakeTrait;
        StateAbbr().fake()
    }

    /// Generate a ZIP code
    pub fn zip() -> String {
        use fake::faker::address::en::ZipCode;
        use fake::Fake as FakeTrait;
        ZipCode().fake()
    }

    /// Generate a country name
    pub fn country() -> String {
        use fake::faker::address::en::CountryName;
        use fake::Fake as FakeTrait;
        CountryName().fake()
    }

    /// Generate a country code
    pub fn country_code() -> String {
        use fake::faker::address::en::CountryCode;
        use fake::Fake as FakeTrait;
        CountryCode().fake()
    }

    /// Generate a latitude coordinate
    pub fn latitude() -> f64 {
        use fake::faker::address::en::Latitude;
        use fake::Fake as FakeTrait;
        Latitude().fake()
    }

    /// Generate a longitude coordinate
    pub fn longitude() -> f64 {
        use fake::faker::address::en::Longitude;
        use fake::Fake as FakeTrait;
        Longitude().fake()
    }

    // ====== Internet ======

    /// Generate a URL
    pub fn url() -> String {
        format!("https://{}", Self::domain())
    }

    /// Generate a domain name
    pub fn domain() -> String {
        use fake::faker::internet::en::DomainSuffix;
        use fake::Fake as FakeTrait;
        let suffix: String = DomainSuffix().fake();
        format!("{}.{}", Self::word().to_lowercase(), suffix)
    }

    /// Generate an IPv4 address
    pub fn ipv4() -> String {
        use fake::faker::internet::en::IPv4;
        use fake::Fake as FakeTrait;
        IPv4().fake()
    }

    /// Generate an IPv6 address
    pub fn ipv6() -> String {
        use fake::faker::internet::en::IPv6;
        use fake::Fake as FakeTrait;
        IPv6().fake()
    }

    /// Generate a MAC address
    pub fn mac_address() -> String {
        use fake::faker::internet::en::MACAddress;
        use fake::Fake as FakeTrait;
        MACAddress().fake()
    }

    /// Generate a user agent string
    pub fn user_agent() -> String {
        use fake::faker::internet::en::UserAgent;
        use fake::Fake as FakeTrait;
        UserAgent().fake()
    }

    /// Generate a password
    pub fn password(min: usize, max: usize) -> String {
        use fake::faker::internet::en::Password;
        use fake::Fake as FakeTrait;
        Password(min..max).fake()
    }

    // ====== Text ======

    /// Generate a random word
    pub fn word() -> String {
        use fake::faker::lorem::en::Word;
        use fake::Fake as FakeTrait;
        Word().fake()
    }

    /// Generate multiple words
    pub fn words(count: usize) -> String {
        use fake::faker::lorem::en::Words;
        use fake::Fake as FakeTrait;
        let words: Vec<String> = Words(count..count + 1).fake();
        words.join(" ")
    }

    /// Generate a sentence
    pub fn sentence() -> String {
        use fake::faker::lorem::en::Sentence;
        use fake::Fake as FakeTrait;
        Sentence(3..10).fake()
    }

    /// Generate multiple sentences
    pub fn sentences(count: usize) -> String {
        use fake::faker::lorem::en::Sentences;
        use fake::Fake as FakeTrait;
        let sentences: Vec<String> = Sentences(count..count + 1).fake();
        sentences.join(" ")
    }

    /// Generate a paragraph
    pub fn paragraph() -> String {
        use fake::faker::lorem::en::Paragraph;
        use fake::Fake as FakeTrait;
        Paragraph(3..7).fake()
    }

    /// Generate multiple paragraphs
    pub fn paragraphs(count: usize) -> String {
        use fake::faker::lorem::en::Paragraphs;
        use fake::Fake as FakeTrait;
        let paragraphs: Vec<String> = Paragraphs(count..count + 1).fake();
        paragraphs.join("\n\n")
    }

    /// Generate a title
    pub fn title() -> String {
        let words = Self::words(Self::number(2, 5) as usize);
        words
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Generate a slug (URL-friendly string)
    pub fn slug() -> String {
        Self::words(Self::number(2, 5) as usize)
            .to_lowercase()
            .replace(' ', "-")
    }

    /// Generate text of specific length
    pub fn text(length: usize) -> String {
        let mut text = String::new();
        while text.len() < length {
            text.push_str(&Self::sentence());
            text.push(' ');
        }
        text.truncate(length);
        text
    }

    // ====== Numbers ======

    /// Generate a random integer between min and max (inclusive)
    pub fn number(min: i32, max: i32) -> i32 {
        rand::thread_rng().gen_range(min..=max)
    }

    /// Generate a random unsigned integer
    pub fn number_u32(min: u32, max: u32) -> u32 {
        rand::thread_rng().gen_range(min..=max)
    }

    /// Generate a random i64
    pub fn number_i64(min: i64, max: i64) -> i64 {
        rand::thread_rng().gen_range(min..=max)
    }

    /// Generate a random float between min and max
    pub fn float(min: f64, max: f64) -> f64 {
        rand::thread_rng().gen_range(min..=max)
    }

    /// Generate a random float with specific decimal places
    pub fn float_with_precision(min: f64, max: f64, decimals: u32) -> f64 {
        let multiplier = 10_f64.powi(decimals as i32);
        (Self::float(min * multiplier, max * multiplier) / multiplier * multiplier).round()
            / multiplier
    }

    /// Generate a random boolean
    pub fn boolean() -> bool {
        rand::thread_rng().gen()
    }

    /// Generate a percentage (0-100)
    pub fn percentage() -> u8 {
        rand::thread_rng().gen_range(0..=100)
    }

    // ====== Dates ======

    /// Generate a random date
    pub fn date() -> NaiveDate {
        Self::date_between(
            NaiveDate::from_ymd_opt(2000, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2024, 12, 31).unwrap(),
        )
    }

    /// Generate a random datetime
    pub fn datetime() -> DateTime<Utc> {
        let date = Self::date();
        let hour = Self::number(0, 23) as u32;
        let minute = Self::number(0, 59) as u32;
        let second = Self::number(0, 59) as u32;

        date.and_hms_opt(hour, minute, second)
            .unwrap()
            .and_local_timezone(Utc)
            .unwrap()
    }

    /// Generate a date between two dates
    pub fn date_between(start: NaiveDate, end: NaiveDate) -> NaiveDate {
        let days = (end - start).num_days();
        let random_days = Self::number_i64(0, days);
        start + Duration::days(random_days)
    }

    /// Generate a date in the past (within specified days)
    pub fn past_date(days: i64) -> NaiveDate {
        Utc::now().date_naive() - Duration::days(Self::number_i64(1, days))
    }

    /// Generate a date in the future (within specified days)
    pub fn future_date(days: i64) -> NaiveDate {
        Utc::now().date_naive() + Duration::days(Self::number_i64(1, days))
    }

    /// Generate a datetime in the past
    pub fn past_datetime(days: i64) -> DateTime<Utc> {
        Utc::now() - Duration::days(Self::number_i64(1, days))
    }

    /// Generate a datetime in the future
    pub fn future_datetime(days: i64) -> DateTime<Utc> {
        Utc::now() + Duration::days(Self::number_i64(1, days))
    }

    // ====== Misc ======

    /// Generate a UUID v4
    pub fn uuid() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Generate a hex color code
    pub fn color_hex() -> String {
        use fake::faker::color::en::HexColor;
        use fake::Fake as FakeTrait;
        HexColor().fake()
    }

    /// Generate a color name
    pub fn color_name() -> String {
        use fake::faker::color::en::Color;
        use fake::Fake as FakeTrait;
        Color().fake()
    }

    /// Generate an image URL (placeholder service)
    pub fn image_url(width: u32, height: u32) -> String {
        format!("https://via.placeholder.com/{}x{}", width, height)
    }

    /// Generate a random file name
    pub fn file_name() -> String {
        let extensions = ["txt", "pdf", "jpg", "png", "doc", "xlsx"];
        let name = Self::word().to_lowercase();
        let ext = Self::random_element(&extensions).unwrap_or(&"txt");
        format!("{}.{}", name, ext)
    }

    /// Generate a file extension
    pub fn file_extension() -> String {
        let extensions = [
            "txt", "pdf", "jpg", "png", "doc", "xlsx", "csv", "json", "xml",
        ];
        Self::random_element(&extensions)
            .unwrap_or(&"txt")
            .to_string()
    }

    /// Generate a file path
    pub fn file_path() -> String {
        let parts = Self::random_elements(&["home", "var", "usr", "tmp", "opt", "etc"], 2);
        let file = Self::file_name();
        format!("/{}/{}", parts.join("/"), file)
    }

    /// Generate a company name
    pub fn company() -> String {
        use fake::faker::company::en::CompanyName;
        use fake::Fake as FakeTrait;
        CompanyName().fake()
    }

    /// Generate a company suffix (Inc, LLC, etc.)
    pub fn company_suffix() -> String {
        use fake::faker::company::en::CompanySuffix;
        use fake::Fake as FakeTrait;
        CompanySuffix().fake()
    }

    /// Generate a job title
    pub fn job_title() -> String {
        use fake::faker::job::en::Title;
        use fake::Fake as FakeTrait;
        Title().fake()
    }

    /// Generate a credit card number
    pub fn credit_card() -> String {
        use fake::faker::creditcard::en::CreditCardNumber;
        use fake::Fake as FakeTrait;
        CreditCardNumber().fake()
    }

    /// Generate a currency code (USD, EUR, etc.)
    pub fn currency_code() -> String {
        use fake::faker::currency::en::CurrencyCode;
        use fake::Fake as FakeTrait;
        CurrencyCode().fake()
    }

    /// Generate a currency name
    pub fn currency_name() -> String {
        use fake::faker::currency::en::CurrencyName;
        use fake::Fake as FakeTrait;
        CurrencyName().fake()
    }

    /// Generate a random element from a slice
    pub fn random_element<T: Clone>(elements: &[T]) -> Option<T> {
        if elements.is_empty() {
            return None;
        }
        let index = rand::thread_rng().gen_range(0..elements.len());
        Some(elements[index].clone())
    }

    /// Generate multiple random elements from a slice
    pub fn random_elements<T: Clone>(elements: &[T], count: usize) -> Vec<T> {
        let mut rng = rand::thread_rng();
        let mut result = Vec::new();
        let max_count = count.min(elements.len());

        for _ in 0..max_count {
            let index = rng.gen_range(0..elements.len());
            result.push(elements[index].clone());
        }

        result
    }

    /// Shuffle a slice
    pub fn shuffle<T>(elements: &mut [T]) {
        use rand::seq::SliceRandom;
        elements.shuffle(&mut rand::thread_rng());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_generation() {
        let name = Fake::name();
        assert!(!name.is_empty());

        let first = Fake::first_name();
        assert!(!first.is_empty());

        let last = Fake::last_name();
        assert!(!last.is_empty());

        let username = Fake::username();
        assert!(!username.is_empty());
    }

    #[test]
    fn test_contact_generation() {
        let email = Fake::email();
        assert!(email.contains('@'));

        let phone = Fake::phone();
        assert!(!phone.is_empty());

        let intl_phone = Fake::phone_international();
        assert!(intl_phone.starts_with('+'));
    }

    #[test]
    fn test_address_generation() {
        let address = Fake::address();
        assert!(!address.is_empty());

        let city = Fake::city();
        assert!(!city.is_empty());

        let state = Fake::state();
        assert!(!state.is_empty());

        let zip = Fake::zip();
        assert!(!zip.is_empty());

        let country = Fake::country();
        assert!(!country.is_empty());
    }

    #[test]
    fn test_internet_generation() {
        let url = Fake::url();
        assert!(url.starts_with("https://"));

        let domain = Fake::domain();
        assert!(domain.contains('.'));

        let ipv4 = Fake::ipv4();
        assert!(!ipv4.is_empty());

        let ipv6 = Fake::ipv6();
        assert!(!ipv6.is_empty());

        let password = Fake::password(8, 16);
        assert!(password.len() >= 8 && password.len() <= 16);
    }

    #[test]
    fn test_text_generation() {
        let word = Fake::word();
        assert!(!word.is_empty());

        let words = Fake::words(5);
        assert!(words.split_whitespace().count() >= 5);

        let sentence = Fake::sentence();
        assert!(!sentence.is_empty());

        let paragraph = Fake::paragraph();
        assert!(!paragraph.is_empty());

        let title = Fake::title();
        assert!(!title.is_empty());

        let slug = Fake::slug();
        assert!(slug.contains('-'));
    }

    #[test]
    fn test_number_generation() {
        let num = Fake::number(1, 100);
        assert!(num >= 1 && num <= 100);

        let float = Fake::float(0.0, 1.0);
        assert!(float >= 0.0 && float <= 1.0);

        let precise = Fake::float_with_precision(0.0, 10.0, 2);
        let decimal_places = precise
            .to_string()
            .split('.')
            .nth(1)
            .map(|s| s.len())
            .unwrap_or(0);
        assert!(decimal_places <= 2);

        let _bool = Fake::boolean();

        let pct = Fake::percentage();
        assert!(pct <= 100);
    }

    #[test]
    fn test_date_generation() {
        use chrono::Datelike;

        let date = Fake::date();
        assert!(date.year() >= 2000);

        let datetime = Fake::datetime();
        assert!(datetime.year() >= 2000);

        let start = NaiveDate::from_ymd_opt(2020, 1, 1).unwrap();
        let end = NaiveDate::from_ymd_opt(2020, 12, 31).unwrap();
        let between = Fake::date_between(start, end);
        assert!(between >= start && between <= end);

        let past = Fake::past_date(30);
        assert!(past < Utc::now().date_naive());

        let future = Fake::future_date(30);
        assert!(future > Utc::now().date_naive());
    }

    #[test]
    fn test_misc_generation() {
        let uuid = Fake::uuid();
        assert_eq!(uuid.len(), 36);

        let hex = Fake::color_hex();
        assert!(hex.starts_with('#'));

        let img = Fake::image_url(800, 600);
        assert!(img.contains("800x600"));

        let company = Fake::company();
        assert!(!company.is_empty());
    }

    #[test]
    fn test_random_element() {
        let elements = vec!["a", "b", "c", "d"];
        let element = Fake::random_element(&elements);
        assert!(element.is_some());
        assert!(elements.contains(&element.unwrap()));

        let empty: Vec<i32> = vec![];
        let none = Fake::random_element(&empty);
        assert!(none.is_none());
    }

    #[test]
    fn test_random_elements() {
        let elements = vec![1, 2, 3, 4, 5];
        let selected = Fake::random_elements(&elements, 3);
        assert_eq!(selected.len(), 3);
    }

    #[test]
    fn test_shuffle() {
        let mut elements = vec![1, 2, 3, 4, 5];
        let original = elements.clone();
        Fake::shuffle(&mut elements);
        // Note: There's a small chance this could fail if shuffle returns same order
        // but for 5 elements, probability is 1/120
        assert_eq!(elements.len(), original.len());
    }
}
