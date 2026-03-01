use std::cell::Cell;
use std::io::Read;
use std::str::FromStr;

use super::*;
use crate::types::*;

thread_local! {
    static TABLE_DEPTH: Cell<u32> = const { Cell::new(0) };
}

const MAX_TABLE_DEPTH: u32 = 32;

struct DepthGuard;

impl DepthGuard {
    fn new() -> Result<Self, ReaderError> {
        TABLE_DEPTH.with(|d| {
            let depth = d.get();
            if depth >= MAX_TABLE_DEPTH {
                return Err(ReaderError::TableDepthExceeded);
            }
            d.set(depth + 1);
            Ok(DepthGuard)
        })
    }
}

impl Drop for DepthGuard {
    fn drop(&mut self) {
        TABLE_DEPTH.with(|d| d.set(d.get() - 1));
    }
}

impl ElementReader for Table {
    fn read<R: Read>(r: &mut EventReader<R>, _: &[OwnedAttribute]) -> Result<Self, ReaderError> {
        let _guard = DepthGuard::new()?;
        let mut t = Table::without_borders(vec![]);
        let mut grid_col: Vec<usize> = vec![];
        loop {
            let e = r.next();
            match e {
                Ok(XmlEvent::StartElement {
                    attributes, name, ..
                }) => {
                    let e = XMLElement::from_str(&name.local_name).unwrap();

                    ignore::ignore_element(e.clone(), XMLElement::TablePropertyChange, r);
                    ignore::ignore_element(e.clone(), XMLElement::TableGridChange, r);

                    match e {
                        XMLElement::TableRow => {
                            t = t.add_row(TableRow::read(r, &attributes)?);
                            continue;
                        }
                        XMLElement::TableWidth => {
                            let (w, width_type) = read_width(&attributes)?;
                            t = t.width(w as usize, width_type);
                            continue;
                        }
                        XMLElement::TableProperty => {
                            if let Ok(p) = TableProperty::read(r, &attributes) {
                                t.property = p;
                            }
                        }
                        XMLElement::Justification => {
                            t = t.align(TableAlignmentType::from_str(&attributes[0].value)?);
                        }
                        XMLElement::TableIndent => {
                            let (w, _) = read_width(&attributes)?;
                            t = t.indent(w as i32);
                            continue;
                        }
                        XMLElement::TableBorders => {
                            if let Ok(borders) = TableBorders::read(r, &attributes) {
                                t = t.set_borders(borders);
                            }
                        }
                        XMLElement::TableStyle => {
                            if let Some(s) = read_val(&attributes) {
                                t = t.style(s);
                            }
                        }
                        XMLElement::TableCellMargin => {
                            if let Ok(margins) = TableCellMargins::read(r, &attributes) {
                                t = t.margins(margins)
                            }
                        }
                        XMLElement::GridCol => {
                            let (w, _) = read_width(&attributes)?;
                            grid_col.push(w as usize);
                        }
                        _ => {}
                    }
                }
                Ok(XmlEvent::EndElement { name, .. }) => {
                    let e = XMLElement::from_str(&name.local_name).unwrap();
                    if e == XMLElement::Table {
                        t = t.set_grid(grid_col);
                        return Ok(t);
                    }
                }
                Err(_) => return Err(ReaderError::XMLReadError),
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_read_table_with_width_prop() {
        let c = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:tbl>
    <w:tblPr>
        <w:tblW w:w="9638" w:type="dxa"/>
    </w:tblPr>
    <w:tblGrid>
        <w:gridCol w:w="3212"/>
        <w:gridCol w:w="3213"/>
        <w:gridCol w:w="3213"/>
    </w:tblGrid>
</w:tbl>
</w:document>"#;
        let mut parser = EventReader::new(c.as_bytes());
        let t = Table::read(&mut parser, &[]).unwrap();
        assert_eq!(
            t,
            Table::without_borders(vec![])
                .set_grid(vec![3212, 3213, 3213])
                .width(9638, WidthType::Dxa)
        );
    }

    #[test]
    fn test_read_table_with_layout() {
        let c = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:tbl>
    <w:tblPr>
        <w:jc w:val="center"/>
        <w:tblInd w:w="100" w:type="dxa"/>
    </w:tblPr>
</w:tbl>
</w:document>"#;
        let mut parser = EventReader::new(c.as_bytes());
        let t = Table::read(&mut parser, &[]).unwrap();
        assert_eq!(
            t,
            Table::without_borders(vec![])
                .align(TableAlignmentType::Center)
                .indent(100)
        );
    }

    #[test]
    fn test_deeply_nested_table_returns_error() {
        // Build XML with nesting deeper than MAX_TABLE_DEPTH (32)
        let depth = 50;
        let mut xml = String::from(
            r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#,
        );
        for _ in 0..depth {
            xml.push_str("<w:tbl><w:tr><w:tc>");
        }
        xml.push_str("<w:p><w:r><w:t>deep</w:t></w:r></w:p>");
        for _ in 0..depth {
            xml.push_str("</w:tc></w:tr></w:tbl>");
        }
        xml.push_str("</w:document>");

        let mut parser = EventReader::new(xml.as_bytes());
        let result = Table::read(&mut parser, &[]);
        assert!(
            result.is_err(),
            "Expected error for deeply nested tables, got Ok"
        );
    }

    #[test]
    fn test_two_level_nested_table_parses_ok() {
        // A normal 2-level nested table should parse fine
        let c = r#"<w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
<w:tbl>
    <w:tr>
        <w:tc>
            <w:tbl>
                <w:tr>
                    <w:tc>
                        <w:p><w:r><w:t>inner</w:t></w:r></w:p>
                    </w:tc>
                </w:tr>
            </w:tbl>
        </w:tc>
    </w:tr>
</w:tbl>
</w:document>"#;
        let mut parser = EventReader::new(c.as_bytes());
        let result = Table::read(&mut parser, &[]);
        assert!(
            result.is_ok(),
            "Expected 2-level nested table to parse OK, got: {:?}",
            result.err()
        );
    }
}
