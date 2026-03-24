use std::time::{Duration, Instant};

use serde_json::{json, Value};
use windows::core::*;
use windows::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows::Win32::System::Com::*;
use windows::Win32::System::Ole::{
    SafeArrayDestroy, SafeArrayGetElement, SafeArrayGetLBound, SafeArrayGetUBound,
};
use windows::Win32::UI::Accessibility::*;

const WINDOW_DISCOVERY_TIMEOUT: Duration = Duration::from_millis(1200);
const WINDOW_DISCOVERY_POLL_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone)]
pub struct ElementInfo {
    pub id: String,
    pub name: String,
    pub role: String,
    pub value: String,
    pub bounds: Rect,
    pub is_enabled: bool,
    pub is_visible: bool,
    pub has_focus: bool,
}

pub struct UiaClient {
    automation: IUIAutomation,
    should_uninitialize_com: bool,
}

impl UiaClient {
    pub async fn new() -> std::result::Result<Self, String> {
        unsafe {
            // Initialize COM for UIA calls. If another COM apartment model is already
            // active on this thread (RPC_E_CHANGED_MODE), continue without
            // uninitializing in Drop because this client did not initialize it.
            let com_init = CoInitializeEx(None, COINIT_MULTITHREADED);
            let should_uninitialize_com = if com_init.is_ok() {
                true
            } else if com_init == RPC_E_CHANGED_MODE {
                false
            } else {
                return Err(format!(
                    "Failed to initialize COM for UI Automation: {}",
                    Error::from(com_init)
                ));
            };

            // Create UI Automation instance
            let automation: IUIAutomation =
                CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
                    .map_err(|e: Error| format!("Failed to create UI Automation: {}", e))?;

            Ok(Self {
                automation,
                should_uninitialize_com,
            })
        }
    }

    pub async fn find_window_by_title(
        &self,
        title: &str,
    ) -> std::result::Result<IUIAutomationElement, String> {
        let title = title.trim();
        if title.is_empty() {
            return Err("Window title cannot be empty".to_string());
        }

        let started = Instant::now();
        loop {
            match self.find_window_by_title_once(title) {
                Ok(element) => return Ok(element),
                Err(err) if started.elapsed() < WINDOW_DISCOVERY_TIMEOUT => {
                    tokio::time::sleep(WINDOW_DISCOVERY_POLL_INTERVAL).await;
                    let _ = err;
                }
                Err(err) => return Err(err),
            }
        }
    }

    fn find_window_by_title_once(
        &self,
        title: &str,
    ) -> std::result::Result<IUIAutomationElement, String> {
        unsafe {
            let root = self
                .automation
                .GetRootElement()
                .map_err(|e: Error| format!("Failed to get root: {}", e))?;

            // Walk through windows to find matching title
            let walker = self
                .automation
                .ControlViewWalker()
                .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

            let mut available_windows: Vec<String> = Vec::new();

            let mut current = walker.GetFirstChildElement(&root).ok();

            while let Some(element) = current {
                let element_name = element.CurrentName().unwrap_or_default();

                let element_name = element_name.to_string();
                if !element_name.trim().is_empty() {
                    available_windows.push(element_name.clone());
                }

                if window_title_matches(&element_name, title) {
                    return Ok(element);
                }

                current = walker.GetNextSiblingElement(&element).ok();
            }

            if available_windows.is_empty() {
                return Err(format!(
                    "Window '{}' not found. UIA did not report any top-level windows.",
                    title
                ));
            }

            available_windows.sort_unstable();
            available_windows.dedup();
            let preview = available_windows
                .into_iter()
                .take(8)
                .collect::<Vec<_>>()
                .join(", ");

            Err(format!(
                "Window '{}' not found. Top-level windows visible to UIA: {}",
                title, preview
            ))
        }
    }

    pub async fn get_ui_tree(&self, window_title: &str) -> std::result::Result<Value, String> {
        let root = self.find_window_by_title(window_title).await?;
        self.serialize_element(&root, 0, 5).await
    }

    fn serialize_element<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        depth: i32,
        max_depth: i32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<Value, String>> + 'a>>
    {
        Box::pin(async move {
            if depth >= max_depth {
                return Ok(json!(null));
            }

            unsafe {
                let name = element.CurrentName().unwrap_or_default().to_string();

                let control_type = element
                    .CurrentControlType()
                    .unwrap_or(UIA_CONTROLTYPE_ID(0));

                let automation_id = element
                    .CurrentAutomationId()
                    .unwrap_or_default()
                    .to_string();

                // Get children
                let walker = self
                    .automation
                    .RawViewWalker()
                    .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

                let mut children = Vec::new();
                let mut child = walker.GetFirstChildElement(element).ok();

                while let Some(c) = child {
                    if let Ok(child_json) = self.serialize_element(&c, depth + 1, max_depth).await {
                        children.push(child_json);
                    }
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Ok(json!({
                    "name": name,
                    "role": control_type_to_string(control_type.0),
                    "automation_id": automation_id,
                    "children": children
                }))
            }
        })
    }

    pub async fn find_elements_by_name(
        &self,
        window_title: &str,
        name: &str,
        exact: bool,
    ) -> std::result::Result<Vec<ElementInfo>, String> {
        let search_name = normalize_non_empty_query(name, "label")?;
        let root = self.find_window_by_title(window_title).await?;
        let mut results = Vec::new();
        self.find_by_name_recursive(&root, &search_name, exact, &mut results)
            .await?;
        Ok(results)
    }

    fn find_by_name_recursive<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        search_name: &'a str,
        exact: bool,
        results: &'a mut Vec<ElementInfo>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<(), String>> + 'a>>
    {
        Box::pin(async move {
            unsafe {
                let element_name = element.CurrentName().unwrap_or_default().to_string();

                let matches = if exact {
                    element_name == search_name
                } else {
                    element_name.contains(search_name)
                };

                if matches {
                    if let Ok(info) = self.element_to_info(element).await {
                        results.push(info);
                    }
                }

                let walker = self
                    .automation
                    .RawViewWalker()
                    .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

                let mut child = walker.GetFirstChildElement(element).ok();
                while let Some(c) = child {
                    self.find_by_name_recursive(&c, search_name, exact, results)
                        .await?;
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Ok(())
            }
        })
    }

    pub async fn find_elements_by_role(
        &self,
        window_title: &str,
        role: &str,
    ) -> std::result::Result<Vec<ElementInfo>, String> {
        let role = normalize_non_empty_query(role, "role")?;
        if !is_supported_role(&role) {
            return Err(format!(
                "Unsupported role '{}'. Use one of: {}",
                role,
                supported_roles_csv()
            ));
        }

        let root = self.find_window_by_title(window_title).await?;
        let mut results = Vec::new();
        self.find_by_role_recursive(&root, &role, &mut results)
            .await?;
        Ok(results)
    }

    fn find_by_role_recursive<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        search_role: &'a str,
        results: &'a mut Vec<ElementInfo>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<(), String>> + 'a>>
    {
        Box::pin(async move {
            unsafe {
                let control_type = element
                    .CurrentControlType()
                    .unwrap_or(UIA_CONTROLTYPE_ID(0));

                let element_role = control_type_to_string(control_type.0);

                if element_role.eq_ignore_ascii_case(search_role) {
                    if let Ok(info) = self.element_to_info(element).await {
                        results.push(info);
                    }
                }

                let walker = self
                    .automation
                    .RawViewWalker()
                    .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

                let mut child = walker.GetFirstChildElement(element).ok();
                while let Some(c) = child {
                    self.find_by_role_recursive(&c, search_role, results)
                        .await?;
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Ok(())
            }
        })
    }

    pub async fn get_element_by_id(
        &self,
        window_title: &str,
        automation_id: &str,
    ) -> std::result::Result<ElementInfo, String> {
        let automation_id = normalize_non_empty_query(automation_id, "element_id")?;
        let root = self.find_window_by_title(window_title).await?;
        self.find_by_id_recursive(&root, &automation_id).await
    }

    fn find_by_id_recursive<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        search_id: &'a str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<ElementInfo, String>> + 'a>,
    > {
        Box::pin(async move {
            unsafe {
                if element_matches_id(element, search_id) {
                    return self.element_to_info(element).await;
                }

                let walker = self
                    .automation
                    .RawViewWalker()
                    .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

                let mut child = walker.GetFirstChildElement(element).ok();
                while let Some(c) = child {
                    if let Ok(info) = self.find_by_id_recursive(&c, search_id).await {
                        return Ok(info);
                    }
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Err(format!("Element with id '{}' not found", search_id))
            }
        })
    }

    async fn element_to_info(
        &self,
        element: &IUIAutomationElement,
    ) -> std::result::Result<ElementInfo, String> {
        unsafe {
            let name = element.CurrentName().unwrap_or_default().to_string();

            let automation_id = element_identifier(element);

            let control_type = element
                .CurrentControlType()
                .unwrap_or(UIA_CONTROLTYPE_ID(0));

            let role = control_type_to_string(control_type.0);

            // Get value using the Value pattern (for text inputs, sliders, etc.)
            let value = match element.GetCurrentPattern(UIA_ValuePatternId) {
                Ok(pattern) => {
                    if let Ok(value_pattern) = pattern.cast::<IUIAutomationValuePattern>() {
                        value_pattern
                            .CurrentValue()
                            .map(|v| v.to_string())
                            .unwrap_or_default()
                    } else {
                        String::new()
                    }
                }
                Err(_) => String::new(),
            };

            // Get bounding rectangle
            let rect =
                element
                    .CurrentBoundingRectangle()
                    .unwrap_or(windows::Win32::Foundation::RECT {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    });

            let bounds = Rect {
                x: rect.left,
                y: rect.top,
                width: rect.right - rect.left,
                height: rect.bottom - rect.top,
            };

            // Get state information
            let is_enabled = element
                .CurrentIsEnabled()
                .unwrap_or(windows::Win32::Foundation::BOOL(0))
                .as_bool();

            let is_visible = !element
                .CurrentIsOffscreen()
                .unwrap_or(windows::Win32::Foundation::BOOL(1))
                .as_bool();

            let has_focus = element
                .CurrentHasKeyboardFocus()
                .unwrap_or(windows::Win32::Foundation::BOOL(0))
                .as_bool();

            Ok(ElementInfo {
                id: automation_id,
                name,
                role,
                value,
                bounds,
                is_enabled,
                is_visible,
                has_focus,
            })
        }
    }

    // High Priority Tool 1: Focus Element
    pub async fn focus_element(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<(), String> {
        unsafe {
            let _element = self.get_element_by_id(window_title, element_id).await?;
            let root = self.find_window_by_title(window_title).await?;

            // Find the actual element by ID again to get IUIAutomationElement
            let found_element = self.find_element_by_id_internal(&root, element_id).await?;

            found_element
                .SetFocus()
                .map_err(|e: Error| format!("Failed to set focus: {}", e))?;

            Ok(())
        }
    }

    // Helper to get IUIAutomationElement by ID
    fn find_element_by_id_internal<'a>(
        &'a self,
        root: &'a IUIAutomationElement,
        search_id: &'a str,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = std::result::Result<IUIAutomationElement, String>>
                + 'a,
        >,
    > {
        Box::pin(async move {
            unsafe {
                if element_matches_id(root, search_id) {
                    return Ok(root.clone());
                }

                let walker = self
                    .automation
                    .RawViewWalker()
                    .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

                let mut child = walker.GetFirstChildElement(root).ok();
                while let Some(c) = child {
                    if let Ok(elem) = self.find_element_by_id_internal(&c, search_id).await {
                        return Ok(elem);
                    }
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Err(format!("Element with id '{}' not found", search_id))
            }
        })
    }

    // High Priority Tool 2: Get Element Value
    pub async fn get_element_value(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<String, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            // Try Value pattern first (text inputs, sliders, etc.)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(value_pattern) = pattern.cast::<IUIAutomationValuePattern>() {
                    if let Ok(value) = value_pattern.CurrentValue() {
                        return Ok(value.to_string());
                    }
                }
            }

            // Try RangeValue pattern (sliders, progress bars)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_RangeValuePatternId) {
                if let Ok(range_pattern) = pattern.cast::<IUIAutomationRangeValuePattern>() {
                    if let Ok(value) = range_pattern.CurrentValue() {
                        return Ok(value.to_string());
                    }
                }
            }

            // Try Toggle pattern (checkboxes)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_TogglePatternId) {
                if let Ok(toggle_pattern) = pattern.cast::<IUIAutomationTogglePattern>() {
                    if let Ok(state) = toggle_pattern.CurrentToggleState() {
                        let state_str = match state.0 {
                            0 => "Off",
                            1 => "On",
                            2 => "Indeterminate",
                            _ => "Unknown",
                        };
                        return Ok(state_str.to_string());
                    }
                }
            }

            // Fallback to CurrentName
            Ok(element.CurrentName().unwrap_or_default().to_string())
        }
    }

    // High Priority Tool 3: Set Element Value
    pub async fn set_element_value(
        &self,
        window_title: &str,
        element_id: &str,
        value: &str,
    ) -> std::result::Result<(), String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            // Try Value pattern (text inputs)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(value_pattern) = pattern.cast::<IUIAutomationValuePattern>() {
                    if let Ok(is_readonly) = value_pattern.CurrentIsReadOnly() {
                        if !is_readonly.as_bool() {
                            let bstr_value = BSTR::from(value);
                            return value_pattern
                                .SetValue(&bstr_value)
                                .map_err(|e: Error| format!("Failed to set value: {}", e));
                        }
                    }
                }
            }

            // Try RangeValue pattern (sliders)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_RangeValuePatternId) {
                if let Ok(range_pattern) = pattern.cast::<IUIAutomationRangeValuePattern>() {
                    if let Ok(parsed_value) = value.parse::<f64>() {
                        return range_pattern
                            .SetValue(parsed_value)
                            .map_err(|e: Error| format!("Failed to set range value: {}", e));
                    }
                }
            }

            Err("Element does not support value setting".to_string())
        }
    }

    // High Priority Tool 4: Type Text
    pub async fn type_text(
        &self,
        window_title: &str,
        element_id: &str,
        text: &str,
    ) -> std::result::Result<(), String> {
        // First focus the element, then set its value
        self.focus_element(window_title, element_id).await?;

        // Small delay to ensure focus is set
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        self.set_element_value(window_title, element_id, text).await
    }

    // High Priority Tool 5: Toggle Checkbox
    pub async fn toggle_checkbox(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<String, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            if let Ok(pattern) = element.GetCurrentPattern(UIA_TogglePatternId) {
                if let Ok(toggle_pattern) = pattern.cast::<IUIAutomationTogglePattern>() {
                    toggle_pattern
                        .Toggle()
                        .map_err(|e: Error| format!("Failed to toggle: {}", e))?;

                    // Return new state
                    if let Ok(state) = toggle_pattern.CurrentToggleState() {
                        let state_str = match state.0 {
                            0 => "Off",
                            1 => "On",
                            2 => "Indeterminate",
                            _ => "Unknown",
                        };
                        return Ok(state_str.to_string());
                    }
                }
            }

            Err("Element does not support toggle pattern".to_string())
        }
    }

    // High Priority Tool 6: Set Checkbox
    pub async fn set_checkbox(
        &self,
        window_title: &str,
        element_id: &str,
        checked: bool,
    ) -> std::result::Result<(), String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            if let Ok(pattern) = element.GetCurrentPattern(UIA_TogglePatternId) {
                if let Ok(toggle_pattern) = pattern.cast::<IUIAutomationTogglePattern>() {
                    // Get current state
                    if let Ok(current_state) = toggle_pattern.CurrentToggleState() {
                        let is_currently_on = current_state.0 == 1;

                        // Toggle if needed to reach desired state
                        if is_currently_on != checked {
                            toggle_pattern
                                .Toggle()
                                .map_err(|e: Error| format!("Failed to toggle: {}", e))?;
                        }

                        return Ok(());
                    }
                }
            }

            Err("Element does not support toggle pattern".to_string())
        }
    }

    // High Priority Tool 7: Select ComboBox Option
    pub async fn select_combobox_option(
        &self,
        window_title: &str,
        element_id: &str,
        option_text: &str,
    ) -> std::result::Result<(), String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            // Expand the combo box
            if let Ok(pattern) = element.GetCurrentPattern(UIA_ExpandCollapsePatternId) {
                if let Ok(expand_pattern) = pattern.cast::<IUIAutomationExpandCollapsePattern>() {
                    expand_pattern
                        .Expand()
                        .map_err(|e: Error| format!("Failed to expand: {}", e))?;

                    // Wait a bit for expansion
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }

            // Find and select the option
            if let Ok(pattern) = element.GetCurrentPattern(UIA_SelectionPatternId) {
                if let Ok(_selection_pattern) = pattern.cast::<IUIAutomationSelectionPattern>() {
                    // Get all items
                    let walker = self
                        .automation
                        .RawViewWalker()
                        .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

                    let mut child = walker.GetFirstChildElement(&element).ok();
                    while let Some(c) = child {
                        let item_name = c.CurrentName().unwrap_or_default().to_string();

                        if item_name == option_text {
                            // Select this item
                            if let Ok(item_pattern) =
                                c.GetCurrentPattern(UIA_SelectionItemPatternId)
                            {
                                if let Ok(select_item) =
                                    item_pattern.cast::<IUIAutomationSelectionItemPattern>()
                                {
                                    select_item.Select().map_err(|e: Error| {
                                        format!("Failed to select item: {}", e)
                                    })?;
                                    return Ok(());
                                }
                            }
                        }

                        child = walker.GetNextSiblingElement(&c).ok();
                    }
                }
            }

            Err(format!("Option '{}' not found in combobox", option_text))
        }
    }

    // Medium Priority Tool 1: Get Checkbox State
    pub async fn get_checkbox_state(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<String, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            if let Ok(pattern) = element.GetCurrentPattern(UIA_TogglePatternId) {
                if let Ok(toggle_pattern) = pattern.cast::<IUIAutomationTogglePattern>() {
                    if let Ok(state) = toggle_pattern.CurrentToggleState() {
                        let state_str = match state.0 {
                            0 => "Off",
                            1 => "On",
                            2 => "Indeterminate",
                            _ => "Unknown",
                        };
                        return Ok(state_str.to_string());
                    }
                }
            }

            Err("Element does not support toggle pattern".to_string())
        }
    }

    // Medium Priority Tool 2: Clear Text
    pub async fn clear_text(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<(), String> {
        self.set_element_value(window_title, element_id, "").await
    }

    // Medium Priority Tool 3: Get Selected Option (from combobox)
    pub async fn get_selected_option(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<String, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            if let Ok(pattern) = element.GetCurrentPattern(UIA_SelectionPatternId) {
                if let Ok(selection_pattern) = pattern.cast::<IUIAutomationSelectionPattern>() {
                    if let Ok(selection) = selection_pattern.GetCurrentSelection() {
                        if let Ok(item) = selection.GetElement(0) {
                            return Ok(item.CurrentName().unwrap_or_default().to_string());
                        }
                    }
                }
            }

            // Fallback: try Value pattern
            if let Ok(pattern) = element.GetCurrentPattern(UIA_ValuePatternId) {
                if let Ok(value_pattern) = pattern.cast::<IUIAutomationValuePattern>() {
                    if let Ok(value) = value_pattern.CurrentValue() {
                        return Ok(value.to_string());
                    }
                }
            }

            Err("Could not get selected option".to_string())
        }
    }

    // Medium Priority Tool 4: Select Tab
    pub async fn select_tab(
        &self,
        window_title: &str,
        tab_name: &str,
    ) -> std::result::Result<(), String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;

            // Find tab items
            let walker = self
                .automation
                .RawViewWalker()
                .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

            self.find_and_select_tab(&root, tab_name, &walker).await
        }
    }

    fn find_and_select_tab<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        tab_name: &'a str,
        walker: &'a IUIAutomationTreeWalker,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<(), String>> + 'a>>
    {
        Box::pin(async move {
            unsafe {
                let control_type = element
                    .CurrentControlType()
                    .unwrap_or(UIA_CONTROLTYPE_ID(0));

                // Check if this is a TabItem
                if control_type.0 == 50019 {
                    // TabItem
                    let name = element.CurrentName().unwrap_or_default().to_string();

                    if name == tab_name {
                        // Select this tab using SelectionItem pattern
                        if let Ok(pattern) = element.GetCurrentPattern(UIA_SelectionItemPatternId) {
                            if let Ok(select_item) =
                                pattern.cast::<IUIAutomationSelectionItemPattern>()
                            {
                                return select_item
                                    .Select()
                                    .map_err(|e: Error| format!("Failed to select tab: {}", e));
                            }
                        }
                    }
                }

                // Recursively search children
                let mut child = walker.GetFirstChildElement(element).ok();
                while let Some(c) = child {
                    if let Ok(_) = self.find_and_select_tab(&c, tab_name, walker).await {
                        return Ok(());
                    }
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Err(format!("Tab '{}' not found", tab_name))
            }
        })
    }

    // Medium Priority Tool 5: Get Active Tab
    pub async fn get_active_tab(&self, window_title: &str) -> std::result::Result<String, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let walker = self
                .automation
                .RawViewWalker()
                .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

            self.find_active_tab(&root, &walker).await
        }
    }

    fn find_active_tab<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        walker: &'a IUIAutomationTreeWalker,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = std::result::Result<String, String>> + 'a>,
    > {
        Box::pin(async move {
            unsafe {
                let control_type = element
                    .CurrentControlType()
                    .unwrap_or(UIA_CONTROLTYPE_ID(0));

                // Check if this is a selected TabItem
                if control_type.0 == 50019 {
                    // TabItem
                    if let Ok(pattern) = element.GetCurrentPattern(UIA_SelectionItemPatternId) {
                        if let Ok(select_item) = pattern.cast::<IUIAutomationSelectionItemPattern>()
                        {
                            if let Ok(is_selected) = select_item.CurrentIsSelected() {
                                if is_selected.as_bool() {
                                    let name =
                                        element.CurrentName().unwrap_or_default().to_string();
                                    return Ok(name);
                                }
                            }
                        }
                    }
                }

                // Recursively search children
                let mut child = walker.GetFirstChildElement(element).ok();
                while let Some(c) = child {
                    if let Ok(tab_name) = self.find_active_tab(&c, walker).await {
                        return Ok(tab_name);
                    }
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Err("No active tab found".to_string())
            }
        })
    }

    // Medium Priority Tool 6: Get Window List
    pub async fn get_window_list(&self) -> std::result::Result<Vec<String>, String> {
        unsafe {
            let root = self
                .automation
                .GetRootElement()
                .map_err(|e: Error| format!("Failed to get root: {}", e))?;

            let walker = self
                .automation
                .RawViewWalker()
                .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

            let mut windows = Vec::new();
            let mut current = walker.GetFirstChildElement(&root).ok();

            while let Some(element) = current {
                let element_name = element.CurrentName().unwrap_or_default().to_string();

                if !element_name.is_empty() {
                    windows.push(element_name);
                }

                current = walker.GetNextSiblingElement(&element).ok();
            }

            Ok(windows)
        }
    }

    // Medium Priority Tool 7: Wait for Element
    pub async fn wait_for_element(
        &self,
        window_title: &str,
        element_id: &str,
        timeout_ms: u64,
    ) -> std::result::Result<ElementInfo, String> {
        let start = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);

        loop {
            // Try to find the element
            match self.get_element_by_id(window_title, element_id).await {
                Ok(element_info) => return Ok(element_info),
                Err(_) => {
                    // Check timeout
                    if start.elapsed() >= timeout {
                        return Err(format!(
                            "Element '{}' not found within {}ms",
                            element_id, timeout_ms
                        ));
                    }

                    // Wait a bit before retrying
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    // Priority 3: Spatial - Find nearest interactive element
    pub async fn find_nearest_interactive_element(
        &self,
        window_title: &str,
        x: i32,
        y: i32,
        max_distance: Option<f64>,
    ) -> std::result::Result<(ElementInfo, f64), String> {
        let interactive = self.find_all_interactive_elements(window_title).await?;
        nearest_element(&interactive, x, y, max_distance).ok_or_else(|| {
            if interactive.is_empty() {
                "No interactive elements found in the window".to_string()
            } else if let Some(max_distance) = max_distance {
                format!(
                    "No interactive element found within {:.1} pixels of ({}, {})",
                    max_distance, x, y
                )
            } else {
                "No nearest element could be determined".to_string()
            }
        })
    }

    // Priority 3: Advanced wait - wait until element bounds stop changing
    pub async fn wait_for_element_stable(
        &self,
        window_title: &str,
        element_id: &str,
        stable_ms: u64,
        timeout_ms: u64,
        poll_interval_ms: u64,
    ) -> std::result::Result<ElementInfo, String> {
        let started = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let stable_for = std::time::Duration::from_millis(stable_ms);
        let poll_interval = std::time::Duration::from_millis(poll_interval_ms.max(10));

        let mut last_bounds: Option<Rect> = None;
        let mut stable_since: Option<std::time::Instant> = None;

        loop {
            if started.elapsed() >= timeout {
                return Err(format!(
                    "Element '{}' did not become stable within {}ms",
                    element_id, timeout_ms
                ));
            }

            match self.get_element_by_id(window_title, element_id).await {
                Ok(element) => {
                    let is_same_bounds = last_bounds
                        .as_ref()
                        .map(|previous| {
                            previous.x == element.bounds.x
                                && previous.y == element.bounds.y
                                && previous.width == element.bounds.width
                                && previous.height == element.bounds.height
                        })
                        .unwrap_or(false);

                    if is_same_bounds {
                        let since = stable_since.get_or_insert_with(std::time::Instant::now);
                        if since.elapsed() >= stable_for {
                            return Ok(element);
                        }
                    } else {
                        stable_since = Some(std::time::Instant::now());
                        last_bounds = Some(element.bounds.clone());
                    }
                }
                Err(_) => {
                    stable_since = None;
                    last_bounds = None;
                }
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    // Priority 3: Advanced wait - wait until element value changes
    pub async fn wait_for_value_change(
        &self,
        window_title: &str,
        element_id: &str,
        initial_value: Option<&str>,
        timeout_ms: u64,
        poll_interval_ms: u64,
    ) -> std::result::Result<String, String> {
        let baseline = match initial_value {
            Some(value) => value.to_string(),
            None => self.get_element_value(window_title, element_id).await?,
        };

        let started = std::time::Instant::now();
        let timeout = std::time::Duration::from_millis(timeout_ms);
        let poll_interval = std::time::Duration::from_millis(poll_interval_ms.max(10));

        loop {
            if started.elapsed() >= timeout {
                return Err(format!(
                    "Element '{}' value did not change from '{}' within {}ms",
                    element_id, baseline, timeout_ms
                ));
            }

            let current = self.get_element_value(window_title, element_id).await?;
            if current != baseline {
                return Ok(current);
            }

            tokio::time::sleep(poll_interval).await;
        }
    }

    // Debug Tool 1: Get Element Path (ancestry chain)
    pub async fn get_element_path(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<Vec<String>, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            let mut path = Vec::new();
            let mut current = Some(element);

            let walker = self
                .automation
                .RawViewWalker()
                .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

            while let Some(elem) = current {
                let name = elem.CurrentName().unwrap_or_default().to_string();
                let automation_id = elem.CurrentAutomationId().unwrap_or_default().to_string();
                let control_type = elem.CurrentControlType().unwrap_or(UIA_CONTROLTYPE_ID(0));
                let role = control_type_to_string(control_type.0);

                let path_element = if !automation_id.is_empty() {
                    format!("{}[{}] (id: {})", role, name, automation_id)
                } else {
                    format!("{}[{}]", role, name)
                };

                path.insert(0, path_element);

                current = walker.GetParentElement(&elem).ok();
            }

            Ok(path)
        }
    }

    // Debug Tool 2: Inspect Element at Point
    pub async fn inspect_element_at_point(
        &self,
        x: i32,
        y: i32,
    ) -> std::result::Result<ElementInfo, String> {
        unsafe {
            let point = windows::Win32::Foundation::POINT { x, y };

            let element = self
                .automation
                .ElementFromPoint(point)
                .map_err(|e: Error| format!("Failed to get element at point: {}", e))?;

            self.element_to_info(&element).await
        }
    }

    // Debug Tool 3: Get Supported Patterns for Element
    pub async fn get_supported_patterns(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<Vec<String>, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            let mut patterns = Vec::new();

            // Check all common patterns
            let pattern_checks = vec![
                (UIA_InvokePatternId, "Invoke"),
                (UIA_ValuePatternId, "Value"),
                (UIA_RangeValuePatternId, "RangeValue"),
                (UIA_TogglePatternId, "Toggle"),
                (UIA_SelectionPatternId, "Selection"),
                (UIA_SelectionItemPatternId, "SelectionItem"),
                (UIA_ExpandCollapsePatternId, "ExpandCollapse"),
                (UIA_ScrollPatternId, "Scroll"),
                (UIA_ScrollItemPatternId, "ScrollItem"),
                (UIA_TextPatternId, "Text"),
                (UIA_WindowPatternId, "Window"),
                (UIA_TransformPatternId, "Transform"),
            ];

            for (pattern_id, pattern_name) in pattern_checks {
                if element.GetCurrentPattern(pattern_id).is_ok() {
                    patterns.push(pattern_name.to_string());
                }
            }

            Ok(patterns)
        }
    }

    // Debug Tool 4: Find All Interactive Elements
    pub async fn find_all_interactive_elements(
        &self,
        window_title: &str,
    ) -> std::result::Result<Vec<ElementInfo>, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let walker = self
                .automation
                .RawViewWalker()
                .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

            let mut interactive_elements = Vec::new();
            self.collect_interactive_elements(&root, &walker, &mut interactive_elements)
                .await?;

            Ok(interactive_elements)
        }
    }

    fn collect_interactive_elements<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        walker: &'a IUIAutomationTreeWalker,
        results: &'a mut Vec<ElementInfo>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<(), String>> + 'a>>
    {
        Box::pin(async move {
            unsafe {
                let control_type = element
                    .CurrentControlType()
                    .unwrap_or(UIA_CONTROLTYPE_ID(0));

                // Check if element is interactive (button, input, checkbox, etc.)
                let interactive_types = vec![
                    50000, // Button
                    50002, // CheckBox
                    50003, // ComboBox
                    50004, // Edit
                    50005, // Hyperlink
                    50007, // ListItem
                    50011, // MenuItem
                    50013, // RadioButton
                    50015, // Slider
                    50016, // Spinner
                    50018, // Tab
                    50019, // TabItem
                ];

                if interactive_types.contains(&control_type.0) {
                    if let Ok(info) = self.element_to_info(element).await {
                        results.push(info);
                    }
                }

                // Recursively check children
                let mut child = walker.GetFirstChildElement(element).ok();
                while let Some(c) = child {
                    self.collect_interactive_elements(&c, walker, results)
                        .await?;
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Ok(())
            }
        })
    }

    // Debug Tool 5: Get Detailed Element Info (extended)
    pub async fn get_element_debug_info(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<Value, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            let info = self.element_to_info(&element).await?;
            let patterns = self
                .get_supported_patterns(window_title, element_id)
                .await
                .unwrap_or_default();
            let path = self
                .get_element_path(window_title, element_id)
                .await
                .unwrap_or_default();

            // Get additional properties
            let class_name = element.CurrentClassName().unwrap_or_default().to_string();
            let accelerator_key = element
                .CurrentAcceleratorKey()
                .unwrap_or_default()
                .to_string();
            let access_key = element.CurrentAccessKey().unwrap_or_default().to_string();
            let help_text = element.CurrentHelpText().unwrap_or_default().to_string();
            let keyboard_focusable = element
                .CurrentIsKeyboardFocusable()
                .unwrap_or(windows::Win32::Foundation::BOOL(0))
                .as_bool();

            Ok(json!({
                "basic_info": {
                    "id": info.id,
                    "name": info.name,
                    "role": info.role,
                    "value": info.value,
                },
                "state": {
                    "enabled": info.is_enabled,
                    "visible": info.is_visible,
                    "focused": info.has_focus,
                    "keyboard_focusable": keyboard_focusable,
                },
                "bounds": {
                    "x": info.bounds.x,
                    "y": info.bounds.y,
                    "width": info.bounds.width,
                    "height": info.bounds.height,
                },
                "properties": {
                    "class_name": class_name,
                    "accelerator_key": accelerator_key,
                    "access_key": access_key,
                    "help_text": help_text,
                },
                "patterns": patterns,
                "path": path,
            }))
        }
    }

    // Debug Tool 6: Dump UI Tree to JSON (with depth limit)
    pub async fn dump_ui_tree_detailed(
        &self,
        window_title: &str,
        max_depth: i32,
    ) -> std::result::Result<Value, String> {
        let root = self.find_window_by_title(window_title).await?;
        self.serialize_element_detailed(&root, 0, max_depth).await
    }

    fn serialize_element_detailed<'a>(
        &'a self,
        element: &'a IUIAutomationElement,
        depth: i32,
        max_depth: i32,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<Value, String>> + 'a>>
    {
        Box::pin(async move {
            if depth >= max_depth {
                return Ok(json!(null));
            }

            unsafe {
                let name = element.CurrentName().unwrap_or_default().to_string();
                let control_type = element
                    .CurrentControlType()
                    .unwrap_or(UIA_CONTROLTYPE_ID(0));
                let automation_id = element
                    .CurrentAutomationId()
                    .unwrap_or_default()
                    .to_string();
                let class_name = element.CurrentClassName().unwrap_or_default().to_string();

                let is_enabled = element
                    .CurrentIsEnabled()
                    .unwrap_or(windows::Win32::Foundation::BOOL(0))
                    .as_bool();
                let is_visible = !element
                    .CurrentIsOffscreen()
                    .unwrap_or(windows::Win32::Foundation::BOOL(1))
                    .as_bool();

                let bounds = element.CurrentBoundingRectangle().unwrap_or(
                    windows::Win32::Foundation::RECT {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    },
                );

                // Get children
                let walker = self
                    .automation
                    .RawViewWalker()
                    .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

                let mut children = Vec::new();
                let mut child = walker.GetFirstChildElement(element).ok();

                while let Some(c) = child {
                    if let Ok(child_json) = self
                        .serialize_element_detailed(&c, depth + 1, max_depth)
                        .await
                    {
                        children.push(child_json);
                    }
                    child = walker.GetNextSiblingElement(&c).ok();
                }

                Ok(json!({
                    "name": name,
                    "role": control_type_to_string(control_type.0),
                    "automation_id": automation_id,
                    "class_name": class_name,
                    "enabled": is_enabled,
                    "visible": is_visible,
                    "bounds": {
                        "x": bounds.left,
                        "y": bounds.top,
                        "width": bounds.right - bounds.left,
                        "height": bounds.bottom - bounds.top,
                    },
                    "children": children,
                }))
            }
        })
    }

    // Debug Tool 7: Get Element Siblings
    pub async fn get_element_siblings(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<Vec<ElementInfo>, String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            let walker = self
                .automation
                .RawViewWalker()
                .map_err(|e: Error| format!("Failed to get walker: {}", e))?;

            let parent = walker
                .GetParentElement(&element)
                .map_err(|e: Error| format!("Failed to get parent: {}", e))?;

            let mut siblings = Vec::new();
            let mut child = walker.GetFirstChildElement(&parent).ok();

            while let Some(c) = child {
                let current_id = c.CurrentAutomationId().unwrap_or_default().to_string();

                // Don't include the element itself
                if current_id != element_id {
                    if let Ok(info) = self.element_to_info(&c).await {
                        siblings.push(info);
                    }
                }

                child = walker.GetNextSiblingElement(&c).ok();
            }

            Ok(siblings)
        }
    }

    // Quick Win 1: Get Center Point
    pub async fn get_center_point(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<(i32, i32), String> {
        let element_info = self.get_element_by_id(window_title, element_id).await?;

        let center_x = element_info.bounds.x + (element_info.bounds.width / 2);
        let center_y = element_info.bounds.y + (element_info.bounds.height / 2);

        Ok((center_x, center_y))
    }

    // Quick Win 2: Click Center
    pub async fn click_center(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<(i32, i32), String> {
        let (x, y) = self.get_center_point(window_title, element_id).await?;
        Ok((x, y))
    }

    // Quick Win 3: Activate Element (using Invoke pattern)
    pub async fn activate_element(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<(), String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            // Try Invoke pattern first (buttons, menu items)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_InvokePatternId) {
                if let Ok(invoke_pattern) = pattern.cast::<IUIAutomationInvokePattern>() {
                    return invoke_pattern
                        .Invoke()
                        .map_err(|e: Error| format!("Failed to invoke element: {}", e));
                }
            }

            // Try Toggle pattern (checkboxes)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_TogglePatternId) {
                if let Ok(toggle_pattern) = pattern.cast::<IUIAutomationTogglePattern>() {
                    return toggle_pattern
                        .Toggle()
                        .map_err(|e: Error| format!("Failed to toggle element: {}", e));
                }
            }

            // Try SelectionItem pattern (list/combo items, tabs)
            if let Ok(pattern) = element.GetCurrentPattern(UIA_SelectionItemPatternId) {
                if let Ok(select_pattern) = pattern.cast::<IUIAutomationSelectionItemPattern>() {
                    return select_pattern
                        .Select()
                        .map_err(|e: Error| format!("Failed to select element: {}", e));
                }
            }

            Err(
                "Element does not support activation (no Invoke, Toggle, or SelectionItem pattern)"
                    .to_string(),
            )
        }
    }

    // Quick Win 4: Get Clipboard
    pub async fn get_clipboard(&self) -> std::result::Result<String, String> {
        use clipboard_win::{formats, get_clipboard};

        get_clipboard::<String, _>(formats::Unicode)
            .map_err(|e| format!("Failed to get clipboard: {}", e))
    }

    // Quick Win 5: Set Clipboard
    pub async fn set_clipboard(&self, text: &str) -> std::result::Result<(), String> {
        use clipboard_win::{formats, set_clipboard};

        set_clipboard(formats::Unicode, text).map_err(|e| format!("Failed to set clipboard: {}", e))
    }

    // Quick Win 6: Scroll to Element
    pub async fn scroll_to_element(
        &self,
        window_title: &str,
        element_id: &str,
    ) -> std::result::Result<(), String> {
        unsafe {
            let root = self.find_window_by_title(window_title).await?;
            let element = self.find_element_by_id_internal(&root, element_id).await?;

            // Try ScrollItem pattern
            if let Ok(pattern) = element.GetCurrentPattern(UIA_ScrollItemPatternId) {
                if let Ok(scroll_pattern) = pattern.cast::<IUIAutomationScrollItemPattern>() {
                    return scroll_pattern
                        .ScrollIntoView()
                        .map_err(|e: Error| format!("Failed to scroll into view: {}", e));
                }
            }

            Err("Element does not support ScrollItem pattern".to_string())
        }
    }
}

impl Drop for UiaClient {
    fn drop(&mut self) {
        if self.should_uninitialize_com {
            unsafe {
                CoUninitialize();
            }
        }
    }
}

const SUPPORTED_ROLES: &[&str] = &[
    "Button",
    "Calendar",
    "CheckBox",
    "ComboBox",
    "Edit",
    "Hyperlink",
    "Image",
    "List",
    "ListItem",
    "Menu",
    "MenuBar",
    "MenuItem",
    "ProgressBar",
    "RadioButton",
    "ScrollBar",
    "Slider",
    "Spinner",
    "StatusBar",
    "Tab",
    "TabItem",
    "Text",
    "ToolBar",
    "ToolTip",
    "Tree",
    "TreeItem",
    "DataGrid",
    "Document",
    "Window",
    "Pane",
    "Table",
    "Group",
];

fn normalize_non_empty_query(input: &str, field_name: &str) -> std::result::Result<String, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(format!("Parameter '{}' cannot be empty", field_name));
    }
    Ok(trimmed.to_string())
}

fn is_supported_role(role: &str) -> bool {
    SUPPORTED_ROLES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(role))
}

fn supported_roles_csv() -> String {
    SUPPORTED_ROLES.join(", ")
}

fn element_matches_id(element: &IUIAutomationElement, search_id: &str) -> bool {
    let search_id = search_id.trim();
    if search_id.is_empty() {
        return false;
    }

    let automation_id = unsafe {
        element
            .CurrentAutomationId()
            .unwrap_or_default()
            .to_string()
            .trim()
            .to_string()
    };

    if !automation_id.is_empty() && automation_id == search_id {
        return true;
    }

    element_identifier(element) == search_id
}

fn element_identifier(element: &IUIAutomationElement) -> String {
    let automation_id = unsafe {
        element
            .CurrentAutomationId()
            .unwrap_or_default()
            .to_string()
            .trim()
            .to_string()
    };

    if !automation_id.is_empty() {
        return automation_id;
    }

    if let Some(runtime_id) = runtime_id_for_element(element) {
        return format!("runtime:{}", runtime_id);
    }

    let name = unsafe { element.CurrentName().unwrap_or_default().to_string() };
    let control_type = unsafe {
        element
            .CurrentControlType()
            .unwrap_or(UIA_CONTROLTYPE_ID(0))
            .0
    };
    let rect = unsafe {
        element
            .CurrentBoundingRectangle()
            .unwrap_or(windows::Win32::Foundation::RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            })
    };

    format!(
        "generated:{}:{}:{}:{}:{}:{}",
        control_type, name, rect.left, rect.top, rect.right, rect.bottom
    )
}

fn runtime_id_for_element(element: &IUIAutomationElement) -> Option<String> {
    let safe_array = unsafe { element.GetRuntimeId().ok()? };
    if safe_array.is_null() {
        return None;
    }

    let result = (|| {
        let lower = unsafe { SafeArrayGetLBound(safe_array, 1).ok()? };
        let upper = unsafe { SafeArrayGetUBound(safe_array, 1).ok()? };
        if upper < lower {
            return None;
        }

        let mut parts = Vec::new();
        for idx in lower..=upper {
            let mut value: i32 = 0;
            if unsafe {
                SafeArrayGetElement(
                    safe_array,
                    &idx,
                    (&mut value as *mut i32).cast::<core::ffi::c_void>(),
                )
                .is_err()
            } {
                return None;
            }
            parts.push(value.to_string());
        }

        Some(parts.join("."))
    })();

    let _ = unsafe { SafeArrayDestroy(safe_array) };
    result
}

fn element_center(bounds: &Rect) -> (i32, i32) {
    (
        bounds.x + (bounds.width / 2),
        bounds.y + (bounds.height / 2),
    )
}

fn euclidean_distance(x1: i32, y1: i32, x2: i32, y2: i32) -> f64 {
    let dx = (x1 - x2) as f64;
    let dy = (y1 - y2) as f64;
    (dx * dx + dy * dy).sqrt()
}

fn nearest_element(
    elements: &[ElementInfo],
    x: i32,
    y: i32,
    max_distance: Option<f64>,
) -> Option<(ElementInfo, f64)> {
    let mut best: Option<(ElementInfo, f64)> = None;

    for element in elements {
        let (cx, cy) = element_center(&element.bounds);
        let distance = euclidean_distance(x, y, cx, cy);
        if max_distance.is_some_and(|limit| distance > limit) {
            continue;
        }

        match &best {
            Some((_, best_distance)) if *best_distance <= distance => {}
            _ => best = Some((element.clone(), distance)),
        }
    }

    best
}

fn control_type_to_string(control_type: i32) -> String {
    // UIA_ControlTypeIds from Windows SDK
    match control_type {
        50000 => "Button".to_string(),
        50001 => "Calendar".to_string(),
        50002 => "CheckBox".to_string(),
        50003 => "ComboBox".to_string(),
        50004 => "Edit".to_string(),
        50005 => "Hyperlink".to_string(),
        50006 => "Image".to_string(),
        50007 => "ListItem".to_string(),
        50008 => "List".to_string(),
        50009 => "Menu".to_string(),
        50010 => "MenuBar".to_string(),
        50011 => "MenuItem".to_string(),
        50012 => "ProgressBar".to_string(),
        50013 => "RadioButton".to_string(),
        50014 => "ScrollBar".to_string(),
        50015 => "Slider".to_string(),
        50016 => "Spinner".to_string(),
        50017 => "StatusBar".to_string(),
        50018 => "Tab".to_string(),
        50019 => "TabItem".to_string(),
        50020 => "Text".to_string(),
        50021 => "ToolBar".to_string(),
        50022 => "ToolTip".to_string(),
        50023 => "Tree".to_string(),
        50024 => "TreeItem".to_string(),
        50025 => "Custom".to_string(),
        50026 => "Group".to_string(),
        50027 => "Thumb".to_string(),
        50028 => "DataGrid".to_string(),
        50029 => "DataItem".to_string(),
        50030 => "Document".to_string(),
        50031 => "SplitButton".to_string(),
        50032 => "Window".to_string(),
        50033 => "Pane".to_string(),
        50034 => "Header".to_string(),
        50035 => "HeaderItem".to_string(),
        50036 => "Table".to_string(),
        50037 => "TitleBar".to_string(),
        50038 => "Separator".to_string(),
        _ => format!("Unknown({})", control_type),
    }
}

fn window_title_matches(candidate: &str, target: &str) -> bool {
    let candidate = candidate.trim();
    let target = target.trim();

    if candidate.is_empty() || target.is_empty() {
        return false;
    }

    if candidate.eq_ignore_ascii_case(target) {
        return true;
    }

    candidate
        .to_ascii_lowercase()
        .contains(&target.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_type_to_string_maps_known_and_unknown_roles() {
        assert_eq!(control_type_to_string(50000), "Button");
        assert_eq!(control_type_to_string(50015), "Slider");
        assert_eq!(control_type_to_string(12345), "Unknown(12345)");
    }

    #[test]
    fn window_title_matches_supports_exact_and_partial_case_insensitive() {
        assert!(window_title_matches("egui-mcp Demo", "egui-mcp demo"));
        assert!(window_title_matches("egui-mcp Demo", "mcp"));
        assert!(!window_title_matches("egui-mcp Demo", "other app"));
        assert!(!window_title_matches("", "demo"));
        assert!(!window_title_matches("egui-mcp Demo", ""));
    }

    #[test]
    fn query_validation_rejects_empty_strings() {
        assert!(normalize_non_empty_query("", "label").is_err());
        assert!(normalize_non_empty_query("   ", "role").is_err());
        assert_eq!(
            normalize_non_empty_query("  Checkbox  ", "label").unwrap(),
            "Checkbox"
        );
    }

    #[test]
    fn supported_role_validation_is_case_insensitive() {
        assert!(is_supported_role("Button"));
        assert!(is_supported_role("button"));
        assert!(!is_supported_role("NotARole"));
    }

    #[test]
    fn nearest_element_picks_closest_center() {
        let elements = vec![
            ElementInfo {
                id: "a".to_string(),
                name: "A".to_string(),
                role: "Button".to_string(),
                value: String::new(),
                bounds: Rect {
                    x: 0,
                    y: 0,
                    width: 20,
                    height: 20,
                },
                is_enabled: true,
                is_visible: true,
                has_focus: false,
            },
            ElementInfo {
                id: "b".to_string(),
                name: "B".to_string(),
                role: "Button".to_string(),
                value: String::new(),
                bounds: Rect {
                    x: 100,
                    y: 100,
                    width: 20,
                    height: 20,
                },
                is_enabled: true,
                is_visible: true,
                has_focus: false,
            },
        ];

        let (nearest, _) = nearest_element(&elements, 5, 5, None).expect("nearest element");
        assert_eq!(nearest.id, "a");
    }

    #[test]
    fn nearest_element_honors_max_distance() {
        let elements = vec![ElementInfo {
            id: "a".to_string(),
            name: "A".to_string(),
            role: "Button".to_string(),
            value: String::new(),
            bounds: Rect {
                x: 100,
                y: 100,
                width: 20,
                height: 20,
            },
            is_enabled: true,
            is_visible: true,
            has_focus: false,
        }];

        let result = nearest_element(&elements, 0, 0, Some(25.0));
        assert!(result.is_none());
    }

    #[tokio::test]
    #[ignore = "requires interactive Windows desktop session with UIA available"]
    async fn uia_client_startup_initializes_automation() {
        let client = UiaClient::new().await;
        assert!(
            client.is_ok(),
            "UIA client should initialize on Windows test hosts: {:?}",
            client.err()
        );
    }

    #[tokio::test]
    #[ignore = "requires running demo-app-win with window title from EGUI_MCP_UIA_TEST_WINDOW"]
    async fn uia_live_window_lookup_and_pattern_support_smoke() {
        let window_title = std::env::var("EGUI_MCP_UIA_TEST_WINDOW")
            .unwrap_or_else(|_| "egui-mcp Demo".to_string());

        let client = UiaClient::new().await.expect("failed to create uia client");
        let _window = client
            .find_window_by_title(&window_title)
            .await
            .expect("failed to find window by title");

        // Prefer the checkbox in demo-app-win because it is expected to have Toggle support.
        let matches = client
            .find_elements_by_name(&window_title, "Checkbox", true)
            .await
            .expect("failed to find checkbox element");
        let checkbox = matches
            .into_iter()
            .find(|element| !element.id.is_empty())
            .expect("checkbox did not expose an automation id");

        let patterns = client
            .get_supported_patterns(&window_title, &checkbox.id)
            .await
            .expect("failed to read supported patterns");

        assert!(
            patterns.iter().any(|pattern| pattern == "Toggle"),
            "expected checkbox to support Toggle pattern, got: {:?}",
            patterns
        );
    }

    #[tokio::test]
    #[ignore = "requires running demo-app-win with window title from EGUI_MCP_UIA_TEST_WINDOW"]
    async fn uia_live_priority2_query_tools_smoke() {
        let window_title = std::env::var("EGUI_MCP_UIA_TEST_WINDOW")
            .unwrap_or_else(|_| "egui-mcp Demo".to_string());

        let client = UiaClient::new().await.expect("failed to create uia client");

        let tree = client
            .get_ui_tree(&window_title)
            .await
            .expect("get_ui_tree failed");
        assert!(tree.is_object(), "expected UI tree object");

        let partial = client
            .find_elements_by_name(&window_title, "Check", false)
            .await
            .expect("find_by_label failed");
        assert!(
            !partial.is_empty(),
            "expected at least one partial label match for 'Check'"
        );

        let exact = client
            .find_elements_by_name(&window_title, "Checkbox", true)
            .await
            .expect("find_by_label_exact failed");
        assert!(
            !exact.is_empty(),
            "expected at least one exact label match for 'Checkbox'"
        );

        let roles = client
            .find_elements_by_role(&window_title, "Slider")
            .await
            .expect("find_by_role failed");
        assert!(
            !roles.is_empty(),
            "expected at least one Slider for find_by_role"
        );

        let element_id = exact
            .into_iter()
            .find(|item| !item.id.trim().is_empty())
            .map(|item| item.id)
            .expect("expected query result with non-empty element id");

        let element = client
            .get_element_by_id(&window_title, &element_id)
            .await
            .expect("get_element failed");
        assert_eq!(element.id, element_id);
    }
}
