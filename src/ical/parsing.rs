fn parse_events(input: &str) -> Vec<ParsedEventBlock> {
    let mut events = Vec::new();
    let mut current: Option<ParsedEventBlock> = None;
    let mut nested_components: Vec<String> = Vec::new();

    for line in unfold_lines(input) {
        let trimmed = line.trim();
        let upper = trimmed.to_ascii_uppercase();

        if let Some(current_event) = current.as_mut() {
            if let Some(component) = upper.strip_prefix("BEGIN:") {
                let component_name = trimmed[6..].trim().to_string();
                nested_components.push(component_name.clone());
                current_event.warnings.push(format!(
                    "Ignored unsupported {} component inside VEVENT.",
                    component_name
                ));
                let _ = component;
                continue;
            }

            if let Some(component) = upper.strip_prefix("END:") {
                if nested_components
                    .last()
                    .map(|active| active.eq_ignore_ascii_case(component))
                    .unwrap_or(false)
                {
                    nested_components.pop();
                    continue;
                }
            }

            if !nested_components.is_empty() {
                continue;
            }
        }

        if upper == "BEGIN:VEVENT" {
            current = Some(ParsedEventBlock::default());
            nested_components.clear();
            continue;
        }

        if upper == "END:VEVENT" {
            if let Some(event) = current.take() {
                events.push(event);
            }
            nested_components.clear();
            continue;
        }

        if let Some(current_event) = current.as_mut() {
            if let Some(property) = parse_property(trimmed) {
                current_event.properties.push(property);
            }
        }
    }

    events
}

fn unfold_lines(input: &str) -> Vec<String> {
    let mut unfolded: Vec<String> = Vec::new();
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    for line in normalized.lines() {
        if line.starts_with(' ') || line.starts_with('\t') {
            if let Some(previous) = unfolded.last_mut() {
                previous.push_str(line[1..].trim_end());
            }
        } else {
            unfolded.push(line.trim_end().to_string());
        }
    }
    unfolded
}

fn parse_property(line: &str) -> Option<ParsedProperty> {
    let (head, value) = line.split_once(':')?;
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_ascii_uppercase();
    let mut params = HashMap::new();
    for param in parts {
        let (key, raw_value) = param.split_once('=').unwrap_or((param, ""));
        params.insert(
            key.trim().to_ascii_uppercase(),
            raw_value.trim().trim_matches('"').to_string(),
        );
    }
    Some(ParsedProperty {
        name,
        params,
        value: value.to_string(),
    })
}
