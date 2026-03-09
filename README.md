# Rust Image FFI Project

Учебный проект по `unsafe` Rust и FFI: CLI-приложение загружает PNG, передаёт буфер пикселей в динамический плагин и сохраняет результат.

## Состав проекта

- `image_processor` — основное CLI-приложение.
- `mirror_plugin` — плагин зеркального отражения.
- `blur_plugin` — плагин размытия.
- `params/` — примеры JSON-файлов параметров.

## Требования

- Rust toolchain (stable)
- Cargo

## Сборка

Собрать всё:

```bash
cargo build
```

Собрать отдельный плагин:

```bash
cargo build -p mirror_plugin
cargo build -p blur_plugin
```

## Запуск image_processor

Аргументы:

- `--input` — путь к входному PNG.
- `--output` — путь к выходному PNG.
- `--plugin` — имя плагина без расширения.
- `--params` — путь к файлу параметров (текст/JSON).
- `--plugin-path` — директория с собранным плагином (по умолчанию `target/debug`).

Пример с зеркальным плагином:

```bash
cargo build --release -p image_processor -p mirror_plugin
./target/release/image_processor \
  --input test_images/src/test.png \
  --output test_images/res/mirror.png \
  --plugin mirror_plugin \
  --params params/mirror_params.json \
  --plugin-path target/release
```

Пример с blur-плагином:

```bash
cargo build --release -p image_processor -p blur_plugin
./target/release/image_processor \
  --input test_images/src/test.png \
  --output test_images/res/blur.png \
  --plugin blur_plugin \
  --params params/blur_params.json \
  --plugin-path target/release
```

## Формат параметров

`mirror_plugin` (`params/mirror_params.json`):

```json
{
  "horizontal": true,
  "vertical": false
}
```

`blur_plugin` (`params/blur_params.json`):

```json
{
  "radius": 2,
  "iterations": 3
}
```

## FFI-контракт плагинов

Оба плагина экспортируют функцию:

```c
void process_image(uint32_t width, uint32_t height, uint8_t* rgba_data, const char* params);
```

Где:

- `rgba_data` — буфер `width * height * 4` байт в формате RGBA8.
- плагин изменяет буфер на месте.
- `params` — NUL-terminated UTF-8 строка с параметрами (обычно JSON).

При невалидных входных данных плагины логируют ошибку в `stderr` и завершаются без паники.

## Тесты

Запуск всех тестов:

```bash
cargo test
```

Тесты по пакетам:

```bash
cargo test -p image_processor
cargo test -p mirror_plugin
cargo test -p blur_plugin
```
