use anyhow::Context;
use log::debug;
use scraper::{ElementRef, Html, Node, Selector};
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::doc;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::Index;
use tantivy::ReloadPolicy;
use tantivy::Searcher;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub word: String,
    pub definition: String,
}

impl TryFrom<ElementRef<'_>> for Entry {
    type Error = anyhow::Error;

    fn try_from(paragraph_el: ElementRef) -> anyhow::Result<Self> {
        debug!(
            "Children {:?}",
            paragraph_el
                .children()
                .map(|c| c.value().as_element())
                .collect::<Vec<_>>()
        );
        let selector = Selector::parse("b").unwrap();
        let word_el = paragraph_el.select(&selector).take(1).next().unwrap();
        let anchor = paragraph_el.first_child().unwrap().value();

        debug!(
            r#"Anchor {:?}
            "#,
            anchor,
        );
        let mut id = "";
        let mut word = "";
        let mut definition = String::new();

        for child in paragraph_el.children() {
            debug!(
                "first child is {:?}",
                child.first_child().map(|c| c.value())
            );
            if let Some(el) = child.value().as_element() {
                if let Some(id_v) = el.attr("id") {
                    if id_v.starts_with("word_") {
                        id = id_v;
                    }
                    if let Some(bold_el) = word_el.first_child() {
                        if let Some(word_text) = bold_el.value().as_text() {
                            word = word_text.trim();
                        }
                    }
                }
            }
            if let Some(txt) = child.first_child().map(|c| c.value()) {
                if let Some(txt_str) = txt.as_text() {
                    definition.push_str(&format!("{} ", txt_str.replace("\n", " ")));
                }
            }
        }

        debug!("ID: {}", id);
        debug!("Word: {}", word);
        debug!("Definition: {}", definition);
        Ok(Entry {
            word: word.to_string(),
            definition: definition.trim().to_owned(),
        })
    }
}

/// A container for indexed words and their definitions.
pub struct Dictionary {
    index: Index,
    searcher: Searcher,
}

impl Dictionary {
    pub fn new(entries: Vec<Entry>) -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("word", TEXT | STORED);
        schema_builder.add_text_field("definition", TEXT | STORED);
        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());
        let mut index_writer = index.writer(50_000_000).context("Couldn't create writer")?;
        let word = schema.get_field("word")?;
        let definition = schema.get_field("definition")?;

        for entry in entries {
            match index_writer.add_document(doc!(
                word => entry.word,
                definition => entry.definition,
            )) {
                Ok(_) => {}
                Err(e) => panic!("{:?}", e),
            }
        }
        index_writer.commit()?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()
            .context("Creating reader")?;
        let searcher = reader.searcher();

        Ok(Dictionary { index, searcher })
    }

    pub fn search(&self, query: &str, limit: Option<usize>) -> anyhow::Result<Vec<Entry>> {
        let word = self
            .index
            .schema()
            .get_field("word")
            .context("Couldn't get word field")?;
        let definition = self
            .index
            .schema()
            .get_field("definition")
            .context("Couldn't get definition field")?;
        let query_parser = QueryParser::for_index(&self.index, vec![word, definition]);
        let query = query_parser.parse_query(&query).context("Invalid query")?;
        let top_docs = self
            .searcher
            .search(&query, &TopDocs::with_limit(limit.unwrap_or(10)))
            .unwrap();
        Ok(top_docs
            .iter()
            .map(|d| {
                let entry = self.searcher.doc(d.1).expect("Failed to retrieve doc");
                let mut word_entries = entry.get_all(word);
                let mut def_entries = entry.get_all(definition);
                Entry {
                    word: word_entries.next().unwrap().as_text().unwrap().to_owned(),
                    definition: def_entries.next().unwrap().as_text().unwrap().to_owned(),
                }
            })
            .collect())
    }

    pub fn define(&self, query: &str) -> anyhow::Result<Vec<Entry>> {
        let word = self
            .index
            .schema()
            .get_field("word")
            .context("Couldn't get word field")?;
        let definition = self
            .index
            .schema()
            .get_field("definition")
            .context("Couldn't get definition field")?;
        let query_parser = QueryParser::for_index(&self.index, vec![word]);
        let query = query_parser.parse_query(&query).context("Invalid query")?;
        let top_docs = self
            .searcher
            .search(&query, &TopDocs::with_limit(10))
            .unwrap();
        Ok(top_docs
            .iter()
            .map(|d| {
                let entry = self.searcher.doc(d.1).expect("Failed to retrieve doc");
                let mut word_entries = entry.get_all(word);
                let mut def_entries = entry.get_all(definition);
                Entry {
                    word: word_entries.next().unwrap().as_text().unwrap().to_owned(),
                    definition: def_entries.next().unwrap().as_text().unwrap().to_owned(),
                }
            })
            .collect())
    }
}

impl TryFrom<Vec<Entry>> for Dictionary {
    type Error = anyhow::Error;

    fn try_from(entries: Vec<Entry>) -> Result<Self, Self::Error> {
        Dictionary::new(entries)
    }
}

/// Parse the given HTML file into a `Vec` of `Entry`. IO or parsing errors may occur.
pub fn parse<P>(file_path: &P) -> anyhow::Result<Dictionary>
where
    P: AsRef<Path>,
{
    let html = std::fs::read_to_string(&file_path)?;
    let document = Html::parse_document(&html);
    let paragraphs = Selector::parse("p").unwrap();

    let entries: Vec<Entry> = document
        .select(&paragraphs)
        .filter(|n| n.has_children())
        .filter(|n| match n.first_child().unwrap().value() {
            Node::Element(e) => {
                if let Some(id) = e.attr("id") {
                    id.starts_with("word_")
                } else {
                    false
                }
            }
            _ => false,
        })
        .map(|n| n.try_into().expect("Invalid element for Entry conversion"))
        .collect();

    Ok(entries.try_into()?)
}

#[cfg(test)]
mod test {
    use super::parse;
    use log::info;
    use std::path::PathBuf;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_parse() {
        init();
        let path = PathBuf::from("data/html/pg31543-images.html");
        info!("Parsing from {:?}", path);
        let dictionary = parse(&path).unwrap();

        let top_docs = dictionary.search("light", None).unwrap();
        assert_eq!(10, top_docs.len());

        //for (score, doc_address) in top_docs {
        //    let retrieved_doc = searcher.doc(doc_address).unwrap();
        //    println!("{score} {:?}", &retrieved_doc);
        //}
    }
}
