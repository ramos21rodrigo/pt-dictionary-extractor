use lexical_sort::{natural_lexical_cmp, StringSort};
use loading::Loading;
use rayon::prelude::*;
use reqwest::Error;
use scraper::{Html, Selector};
use std::{
    fs::File,
    io::{self, Write},
    sync::{Arc, Mutex},
};

const URL: &str = "http://www.portaldalinguaportuguesa.org/advanced.php?action=browse";
const FILE_NAME: &str = "dictionary.txt";

#[tokio::main]
async fn get_document(fist_letter: char, second_letter: char) -> Result<Html, Error> {
    let html = reqwest::get(format!("{}&l1={}&l2={}", URL, fist_letter, second_letter))
        .await?
        .text()
        .await?;
    Ok(Html::parse_document(&html))
}

fn parse(pages: &Arc<Mutex<Vec<String>>>, document: Html) {
    let maintext_selector = Selector::parse("td#maintext").unwrap();
    let fourth_td_selector = Selector::parse("td:nth-child(4)").unwrap();
    let a_selector = Selector::parse("a").unwrap();
    let mut page = pages.lock().unwrap();

    let Some(maintext) = document.select(&maintext_selector).next() else {
        panic!("Element td#maintext not found");
    };
    let Some(fourth_td) = maintext.select(&fourth_td_selector).next() else {
        panic!("Element missing");
    };
    for element in fourth_td.select(&a_selector) {
        page.push(element.inner_html())
    }
}

fn write_dictionary(dictionary: &Vec<String>) -> io::Result<()> {
    let mut file = File::create(FILE_NAME)?;

    for word in dictionary {
        writeln!(&mut file, "{}", word)?;
    }
    Ok(())
}

fn main() {
    let loading = Loading::default();
    let alphabet = "abcdefghijklmnopqrstuvwxyz";
    let alphabet_chars: Vec<char> = alphabet.chars().collect();

    loading.text("Extracting words");
    let pages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(vec![]));
    alphabet_chars.par_iter().for_each(|&c1| {
        alphabet_chars.par_iter().for_each(|&c2| {
            let document = match get_document(c1, c2) {
                Ok(doc) => doc,
                Err(error) => {
                    loading.fail("Extracting words");
                    panic!("Error: {}", error)
                }
            };
            parse(&pages, document);
        });
    });
    loading.success("Extracting words");

    loading.text("Sorting results");
    let mut dictionary = pages.lock().unwrap().clone();
    dictionary.dedup();
    dictionary.string_sort_unstable(natural_lexical_cmp);
    loading.success("Sorting results");

    loading.text("Creating dictionary");
    match write_dictionary(&dictionary) {
        Ok(()) => loading.success("Creating dictionary"),
        Err(error) => {
            loading.success("Creating dictionary");
            eprintln!("Error: {}", error)
        }
    };
}
