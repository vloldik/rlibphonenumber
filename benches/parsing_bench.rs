// benches/parsing_benchmark.rs

use criterion::{black_box, criterion_group, criterion_main, Criterion};

// --- Импорты из вашей библиотеки ---
use rlibphonenumber::PHONE_NUMBER_UTIL;

// --- Импорты из внешней библиотеки ---
use phonenumber::{self as rlp, country::Id};

// Тип для наших тестовых данных: (строка_номера, регион_для_вас, регион_для_rlp)
type TestEntity = (&'static str, &'static str, Id);

/// Подготавливает разнообразный набор данных для тестирования парсинга.
/// Это дает более объективную оценку, чем один номер.
fn setup_parsing_data() -> Vec<TestEntity> {
    use phonenumber::country::Id::*;
    vec![
        // Оригинальный номер из вашего примера
        ("0011 54 9 11 8765 4321 ext. 1234", "AU", AU),
        // Простой номер США в национальном формате
        ("(650) 253-0000", "US", US),
        // Номер Великобритании в международном формате со знаком +
        ("+44 20 8765 4321", "GB", GB),
        // Номер Великобритании с национальным префиксом (ведущий ноль)
        ("020 8765 4321", "GB", GB),
        // Сложный мобильный номер Аргентины
        ("011 15-1234-5678", "AR", AR),
        // Итальянский номер со значащим ведущим нулем
        ("02 12345678", "IT", IT),
        // "Vanity" номер (с буквами)
        ("1-800-FLOWERS", "US", US),
        // Короткий номер, который может быть валидным в некоторых регионах
        ("12345", "DE", DE),
    ]
}

fn parsing_benchmark(c: &mut Criterion) {
    // Получаем наш набор тестовых данных
    let numbers_to_parse = setup_parsing_data();

    let mut group = c.benchmark_group("Parsing Comparison");

    // --- Бенчмарк для вашей библиотеки rlibphonenumber ---
    group.bench_function("rlibphonenumber: parse()", |b| {
        // b.iter() запускает код в цикле много раз для замера
        b.iter(|| {
            // Итерируемся по всем номерам в нашем наборе
            for (number_str, region, _) in &numbers_to_parse {
                // Вызываем parse, обернув аргументы в black_box.
                // Это гарантирует, что компилятор не оптимизирует вызов.
                // Мы не используем результат, так как нас интересует только скорость выполнения.
                let _ = PHONE_NUMBER_UTIL.parse(black_box(number_str), black_box(region));
            }
        })
    });

    // --- Бенчмарк для библиотеки rust-phonenumber ---
    group.bench_function("rust-phonenumber: parse()", |b| {
        b.iter(|| {
            for (number_str, _, region_id) in &numbers_to_parse {
                // Аналогичный вызов для второй библиотеки
                let _ = rlp::parse(black_box(Some(*region_id)), black_box(number_str));
            }
        })
    });

    group.finish();
}

// Макросы для регистрации и запуска бенчмарка
criterion_group!(benches, parsing_benchmark);
criterion_main!(benches);