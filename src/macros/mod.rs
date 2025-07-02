// std::borrow::Cow
// std::option::Option

/// This macro extracts owned value from cow
/// but if cow is borrowed it returns default given value
/// 
/// it's helpful when function returns `Cow<'_, T>` as result,
/// where `Cow::Borrowed` option marks that value was not modified
/// and we can use owned original instead of copying it.
macro_rules! owned_from_cow_or {
    ($getcow:expr, $default:expr) => {{
        if let std::borrow::Cow::Owned(s) = $getcow {
            s
        } else {
            $default
        }
    }};
}

pub(crate) use owned_from_cow_or;

