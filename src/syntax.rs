use crate::app::StateCraftApp;

pub fn run(app: &mut StateCraftApp, script: &str) {
    for raw in script.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('*') {
            continue;
        }
        let up = line.to_uppercase();
        if up.starts_with("LOAD ") || up.starts_with("GET FILE") {
            // extract path
            let path = extract_path(line);
            app.path_input = path;
            app.load();
        } else if up == "DESCRIPTIVES" || up.starts_with("DESCRIPTIVES ") {
            app.analyze_descriptives();
        } else if up == "FREQUENCIES" || up.starts_with("FREQUENCIES ") {
            app.analyze_frequencies();
        } else if up == "CORRELATIONS" || up.starts_with("CORRELATIONS ") {
            app.analyze_correlations();
        } else if up == "REGRESSION" || up.starts_with("REGRESSION ") {
            app.analyze_regression();
        } else if up == "TTEST" || up.starts_with("T-TEST") || up.starts_with("T TEST") {
            app.analyze_ttest();
        } else if up == "CLEAR" || up == "CLEAR OUTPUT" {
            app.outputs.clear();
            app.status = "Output cleared.".to_string();
        } else {
            app.status = format!("Unknown command: {}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::StateCraftApp;

    #[test]
    fn run_basic_script() {
        let mut app = StateCraftApp::default();
        let script = "LOAD \"samples/demo.csv\"\nDESCRIPTIVES\nFREQUENCIES\nCORRELATIONS\nREGRESSION\nCLEAR\n";
        run(&mut app, script);
        assert!(app.table.is_some());
    }
}

fn extract_path(line: &str) -> String {
    // quoted string
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    if let Some(start) = line.find('\'') {
        if let Some(end) = line[start + 1..].find('\'') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    // last token
    line.split_whitespace().last().unwrap_or("").to_string()
}