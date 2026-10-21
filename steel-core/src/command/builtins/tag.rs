//! Vanilla entity scoreboard tag command.

use std::collections::BTreeSet;

use steel_utils::{Identifier, translations};
use text_components::{Modifier, TextComponent, format::Color, translation::Translation};

use super::super::{
    brigadier::{ArgumentType, CommandNodeBuilder, CommandSyntaxError},
    execution::{
        CommandSource, SteelArgumentType, SteelCommandContext, SteelCommandRuntime, argument,
        literal,
    },
    registration::CommandRegistration,
};
use crate::entity::SharedEntity;

pub(super) fn registration() -> CommandRegistration<CommandSource> {
    CommandRegistration::new(Identifier::vanilla_static("tag"), |_| command())
}

fn command() -> CommandNodeBuilder<CommandSource, SteelCommandRuntime> {
    literal("tag").then(
        argument("targets", SteelArgumentType::entities())
            .then(literal("add").then(argument("name", ArgumentType::word()).executes(add_tag)))
            .then(
                literal("remove")
                    .then(argument("name", SteelArgumentType::entity_tag()).executes(remove_tag)),
            )
            .then(literal("list").executes(list_tags)),
    )
}

/// Counts the elements with a non-zero result and remembers the only one, so
/// feedback can name a lone affected element instead of a count.
struct ResponseTracker<'a, T> {
    non_zero_count: usize,
    first_non_zero: Option<&'a T>,
}

impl<'a, T> ResponseTracker<'a, T> {
    const fn new() -> Self {
        Self {
            non_zero_count: 0,
            first_non_zero: None,
        }
    }

    const fn track(&mut self, element: &'a T, non_zero: bool) {
        if !non_zero {
            return;
        }
        if self.non_zero_count == 0 {
            self.first_non_zero = Some(element);
        }
        self.non_zero_count += 1;
    }

    const fn only_non_zero(&self) -> Option<&'a T> {
        if self.non_zero_count == 1 {
            self.first_non_zero
        } else {
            None
        }
    }

    fn total(&self) -> i32 {
        i32::try_from(self.non_zero_count).unwrap_or(i32::MAX)
    }
}

fn add_tag(context: &SteelCommandContext<CommandSource>) -> Result<i32, CommandSyntaxError> {
    let name = context.string("name")?;
    change_tags(
        context,
        &translations::COMMANDS_TAG_ADD_FAILED,
        &translations::COMMANDS_TAG_ADD_SUCCESS_SINGLE,
        &translations::COMMANDS_TAG_ADD_SUCCESS_MULTIPLE,
        name,
        |target| target.add_tag(name.to_owned()),
    )
}

fn remove_tag(context: &SteelCommandContext<CommandSource>) -> Result<i32, CommandSyntaxError> {
    let name = context.string("name")?;
    change_tags(
        context,
        &translations::COMMANDS_TAG_REMOVE_FAILED,
        &translations::COMMANDS_TAG_REMOVE_SUCCESS_SINGLE,
        &translations::COMMANDS_TAG_REMOVE_SUCCESS_MULTIPLE,
        name,
        |target| target.remove_tag(name),
    )
}

fn change_tags(
    context: &SteelCommandContext<CommandSource>,
    failed: &Translation<0>,
    single: &Translation<2>,
    multiple: &Translation<2>,
    name: &str,
    change: impl Fn(&SharedEntity) -> bool,
) -> Result<i32, CommandSyntaxError> {
    let targets = context.entities("targets")?;
    let mut tracker = ResponseTracker::new();
    for target in &targets {
        tracker.track(target, change(target));
    }

    if tracker.non_zero_count == 0 {
        return Err(CommandSyntaxError::dynamic(TextComponent::from(failed)));
    }

    let message = match tracker.only_non_zero() {
        Some(target) => single
            .message([TextComponent::plain(name.to_owned()), target.display_name()])
            .component(),
        None => multiple
            .message([
                TextComponent::plain(name.to_owned()),
                TextComponent::plain(tracker.non_zero_count.to_string()),
            ])
            .component(),
    };
    context.source().send_success(&message, true);

    Ok(tracker.total())
}

fn list_tags(context: &SteelCommandContext<CommandSource>) -> Result<i32, CommandSyntaxError> {
    let targets = context.entities("targets")?;
    let mut tracker = ResponseTracker::new();
    let mut tags = BTreeSet::new();
    for target in &targets {
        let entity_tags = target.tags();
        tracker.track(target, !entity_tags.is_empty());
        tags.extend(entity_tags);
    }

    // The empty feedback also goes through the non-zero tracker, so it is
    // always the multiple form, even for a single target.
    let message = if tags.is_empty() {
        translations::COMMANDS_TAG_LIST_MULTIPLE_EMPTY
            .message([TextComponent::plain(tracker.non_zero_count.to_string())])
            .component()
    } else if let Some(target) = tracker.only_non_zero() {
        translations::COMMANDS_TAG_LIST_SINGLE_SUCCESS
            .message([
                target.display_name(),
                TextComponent::plain(tags.len().to_string()),
                format_list(&tags),
            ])
            .component()
    } else {
        translations::COMMANDS_TAG_LIST_MULTIPLE_SUCCESS
            .message([
                TextComponent::plain(tracker.non_zero_count.to_string()),
                TextComponent::plain(tags.len().to_string()),
                format_list(&tags),
            ])
            .component()
    };
    context.source().send_success(&message, false);

    Ok(i32::try_from(tags.len()).unwrap_or(i32::MAX))
}

/// Sorted, green values joined by a gray separator.
fn format_list(values: &BTreeSet<String>) -> TextComponent {
    let mut list = TextComponent::plain("");
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            list = list.add_child(TextComponent::plain(", ").color(Color::Gray));
        }
        list = list.add_child(TextComponent::plain(value.clone()).color(Color::Green));
    }
    list
}

#[cfg(test)]
mod tests {
    use super::ResponseTracker;

    #[test]
    fn tracker_only_reports_a_lone_non_zero_element() {
        let elements = ["a", "b", "c"];

        let mut tracker = ResponseTracker::new();
        for element in &elements {
            tracker.track(element, false);
        }
        assert_eq!((tracker.total(), tracker.only_non_zero()), (0, None));

        let mut tracker = ResponseTracker::new();
        tracker.track(&elements[0], false);
        tracker.track(&elements[1], true);
        tracker.track(&elements[2], false);
        assert_eq!(tracker.total(), 1);
        assert_eq!(tracker.only_non_zero(), Some(&"b"));

        tracker.track(&elements[2], true);
        assert_eq!((tracker.total(), tracker.only_non_zero()), (2, None));
    }
}
