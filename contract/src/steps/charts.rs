#![cfg(test)]

use std::env::current_dir;

use fake::Fake;
use near_sdk::NearToken;
use plotters::{
    backend::BitMapBackend,
    chart::{ChartBuilder, ChartContext, LabelAreaPosition, SeriesLabelPosition},
    coord::types::{RangedCoordu128, RangedCoordu64},
    drawing::IntoDrawingArea,
    element::PathElement,
    prelude::{Cartesian2d, LineSeries, RGBAColor, BLACK, BLUE, MAGENTA, WHITE},
    style::Color,
};
use sweat_jar_model::jar::JarId;

use crate::{
    test_builder::{ProductField::*, TestAccess, TestBuilder},
    test_utils::STEPS_PRODUCT,
};

struct AccrualsData {
    day: u64,
    steps: u128,
    simple: u128,
}

fn get_data() -> Vec<AccrualsData> {
    const JAR: JarId = 0;
    const STEP_JAR: JarId = 1;

    let mut context = TestBuilder::new()
        .product_build(STEPS_PRODUCT, [APY(0), StepsCap(20_000)])
        .jar(STEP_JAR)
        .product(12)
        .jar(JAR)
        .build();

    let mut result = vec![];

    for day in 1..400 {
        context.set_block_timestamp_in_days(day);

        if day < 100 {
            context.record_steps(1, (4_000..10_000).fake());
        } else {
            context.record_steps(1, (15_000..20_000).fake());
        }

        result.push(AccrualsData {
            day,
            steps: context.interest(STEP_JAR),
            simple: context.interest(JAR),
        });
    }

    result
}

#[test]
#[ignore]
fn plot() -> anyhow::Result<()> {
    render_chart("Step jars interest", get_data(), "walk.png")?;

    Ok(())
}

fn render_chart(name: &str, data: Vec<AccrualsData>, file_name: &str) -> anyhow::Result<()> {
    let current_dir = current_dir().unwrap();
    let root_dir = current_dir.parent().unwrap();
    let output_file_path = format!("{}/docs/{file_name}", root_dir.display());

    let root = BitMapBackend::new(output_file_path.as_str(), (2800, 1800)).into_drawing_area();

    root.fill(&WHITE)?;

    let min_x: u64 = data.iter().map(|data| data.day).min().unwrap();
    let max_x: u64 = data.iter().map(|data| data.day).max().unwrap();

    let min_y: u128 = data.iter().map(|data| data.steps).min().unwrap();
    let max_y: u128 = data.iter().map(|data| data.simple).max().unwrap() + NearToken::from_near(12).as_yoctonear();

    let mut chart = ChartBuilder::on(&root)
        .set_label_area_size(LabelAreaPosition::Left, 150)
        .set_label_area_size(LabelAreaPosition::Bottom, 100)
        .margin(10)
        .caption(name, ("sans-serif", 60))
        .build_cartesian_2d(min_x..max_x, min_y..max_y)?;

    chart
        .configure_mesh()
        .y_label_style(("sans-serif", 40))
        .x_label_style(("sans-serif", 40))
        .x_desc("Days")
        .y_desc("$SWEAT / APY %")
        .y_label_formatter(&|value| format!("{:.1}", *value as f64 / NearToken::from_near(1).as_yoctonear() as f64))
        .draw()?;

    // draw_graph(
    //     &mut chart,
    //     data.iter().map(|data| {
    //         (
    //             data.day,
    //             (data.apy * 10000.0) as u128 * NearToken::from_near(1).as_yoctonear() / 100,
    //         )
    //     }),
    //     "APY",
    //     GREEN,
    // )?;

    draw_graph(
        &mut chart,
        data.iter().map(|data| (data.day, data.steps)),
        "1% - 1000 steps",
        BLUE,
    )?;

    draw_graph(
        &mut chart,
        data.iter().map(|data| (data.day, data.simple)),
        "12% jar",
        MAGENTA.mix(0.5),
    )?;

    chart
        .configure_series_labels()
        .position(SeriesLabelPosition::UpperLeft)
        .legend_area_size(140)
        .label_font(("sans-serif", 60))
        .border_style(BLACK.stroke_width(4))
        .draw()?;

    root.present().expect("Unable to write result to file");
    Ok(())
}

fn draw_graph(
    chart: &mut ChartContext<BitMapBackend, Cartesian2d<RangedCoordu64, RangedCoordu128>>,
    data: impl IntoIterator<Item = (u64, u128)>,
    label: &str,
    color: impl Into<RGBAColor>,
) -> anyhow::Result<()> {
    let color: RGBAColor = color.into();
    let series = LineSeries::new(data, color.stroke_width(8));
    chart
        .draw_series(series)?
        .label(label)
        .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 100, y)], color.stroke_width(8)));

    Ok(())
}
