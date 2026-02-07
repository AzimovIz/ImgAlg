use anyhow::{bail, Context, Result};
use image::{DynamicImage, GenericImageView, Rgba};
use std::collections::HashMap;
use std::env;

/// Функция преобразования изображения в единый формат RGBA
fn convert_to_rgba(sample_img: DynamicImage) -> DynamicImage {
    match sample_img {
        DynamicImage::ImageRgb8(_) => image::DynamicImage::ImageRgba8(sample_img.into_rgba8()),
        DynamicImage::ImageRgba8(_) => sample_img.clone(), // Уже в RGBA
        DynamicImage::ImageLuma8(_) => image::DynamicImage::ImageRgba8(sample_img.into_rgba8()),
        DynamicImage::ImageLumaA8(_) => image::DynamicImage::ImageRgba8(sample_img.into_rgba8()),
        _ => panic!("Неподдерживаемый формат изображения."),
    }
}

pub struct ImagesComparer {
    pub compare_with_first: bool,
    images: Vec<(Vec<Vec<i32>>, HashMap<usize, i32>)>,
}

impl ImagesComparer {
    pub fn new(images: &[&String]) -> Result<Self> {
        let mut imgs = vec![];
        for img in images.iter().copied() {
            let diff_pixels = Self::_get_pixels_diff(img)?;
            imgs.push((diff_pixels, Default::default()));
        }
        Ok(Self { compare_with_first: false, images: imgs})
    }

    fn _get_image_type(image_path: &str) -> Result<String> {
        let reader = image::ImageReader::open(image_path)?
                            .with_guessed_format()?
                            .decode()?;
        match reader.color() {
            image::ColorType::Rgb8 => Ok("jpg".to_string()), // JPEG поддерживает RGB
            image::ColorType::Rgba8 => Ok("png".to_string()), // PNG поддерживает RGBA
            image::ColorType::L8 => Ok("gray".to_string()), // Grayscale изображения
            _ => bail!("Unsupported image format"),
        }
    }

    /// Новая функция обработки пикселей с предварительным преобразованием
    fn _get_pixels_diff(image_path: &str) -> Result<Vec<Vec<i32>>> {
        let original_img = image::open(image_path).context("Failed to open the image")?;
        let converted_img = convert_to_rgba(original_img); // Конвертируем изображение в RGBA
        let scaled_sample = converted_img.resize_exact(16, 16, image::imageops::FilterType::Gaussian);
        let pixels = scaled_sample.pixels().collect::<Vec<_>>();

        let mut result = vec![];
        let mut prev_color = None;
        for y in 0..16 {
            for x in 0..16 {
                let pixel = *pixels.get(y * 16 + x).unwrap_or(&(0, 0, Rgba([0, 0, 0, 255]))); // Дефолтный прозрачный пиксель
                let color = [
                    (pixel.2[0] as i32).pow(2), // Первая составляющая (красный)
                    (pixel.2[1] as i32).pow(2), // Вторая составляющая (зеленый)
                    (pixel.2[2] as i32).pow(2), // Третья составляющая (синий)
                ];
                if Some(color) != prev_color && !(x == 0 && y == 0) {
                    result.push(vec![
                        color[0] - prev_color.unwrap()[0], // Преобразовываем в вектор
                        color[1] - prev_color.unwrap()[1],
                        color[2] - prev_color.unwrap()[2],
                    ]);
                }
                prev_color = Some(color);
            }
        }
        Ok(result)
    }

    fn _get_diff(&self) -> f32 {
        let mut diff = 0.0;
        for i in 0..std::cmp::min(self.images[0].0.len(), self.images[1].0.len()) {
            diff += ((self.images[0].0[i][0] - self.images[1].0[i][0]) as f32 ).abs().sqrt();
            diff += ((self.images[0].0[i][1] - self.images[1].0[i][1]) as f32 ).abs().sqrt();
            diff += ((self.images[0].0[i][2] - self.images[1].0[i][2]) as f32 ).abs().sqrt();
        }
        diff
    }

    /// Новый метод для получения процента схожести
    pub fn similarity_percentage(&self) -> f32 {
        let total_difference = self._get_diff() as f64;
        let num_pixels = (16 * 16) as f64; 
        let max_possible_difference_per_channel = 100.0; // Максимально возможное отличие в каждом канале
        let channels_count = 3.0; // Три канала (RGB)
        let max_total_difference = num_pixels * channels_count * max_possible_difference_per_channel;
        let percentage_similarity = 100.0 - (total_difference / max_total_difference) * 100.0;
        (percentage_similarity as f32).clamp(0.0, 100.0) // Ограничиваем диапазон от 0% до 100%
    }

    pub fn compare(&mut self) {
        let diff = self._get_diff() as i32;
        self.images[0].1.insert(1, diff); // Храним разницу между первыми двумя изображениями
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let images = &[&args[1], &args[2]];

    // Создаем объект сравнителя изображений
    let mut comparer = match ImagesComparer::new(images) {
        Ok(comparer) => comparer,
        Err(e) => {
            eprintln!("Ошибка при создании компаратора: {}", e);
            return;
        }
    };

    // Проверяем наличие хотя бы двух изображений
    if comparer.images.is_empty() {
        eprintln!("Нет изображений для сравнения!");
        return;
    }

    // Запускаем процесс сравнения
    comparer.compare();

    // Выводим результат сравнения
    println!("Results:");
    for (idx, data) in comparer.images.iter().enumerate() {
        println!("Image {}: {:?}", idx, data.1); // Выводим метаданные сравнения
    }

    // Выводим процент схожести
    let percent_similarity = comparer.similarity_percentage();
    println!("Процент схожести: {:.2}%", percent_similarity);
}