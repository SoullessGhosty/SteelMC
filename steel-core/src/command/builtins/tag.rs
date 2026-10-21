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
    let changed: Vec<_> = targets.iter().filter(|target| change(target)).collect();

    let message = match changed.as_slice() {
        [] => return Err(CommandSyntaxError::dynamic(TextComponent::from(failed))),
        [target] => single
            .message([TextComponent::plain(name.to_owned()), target.display_name()])
            .component(),
        _ => multiple
            .message([
                TextComponent::plain(name.to_owned()),
                TextComponent::plain(changed.len().to_string()),
            ])
            .component(),
    };
    context.source().send_success(&message, true);

    Ok(i32::try_from(changed.len()).unwrap_or(i32::MAX))
}

fn list_tags(context: &SteelCommandContext<CommandSource>) -> Result<i32, CommandSyntaxError> {
    let targets = context.entities("targets")?;
    let tagged: Vec<_> = targets
        .iter()
        .filter(|target| !target.tags().is_empty())
        .collect();
    let tags: BTreeSet<String> = tagged.iter().flat_map(|target| target.tags()).collect();

    // Vanilla's empty feedback counts tagged targets, so it is always the
    // multiple form, even for a single target.
    let message = match tagged.as_slice() {
        [] => translations::COMMANDS_TAG_LIST_MULTIPLE_EMPTY
            .message([TextComponent::plain("0")])
            .component(),
        [target] => translations::COMMANDS_TAG_LIST_SINGLE_SUCCESS
            .message([
                target.display_name(),
                TextComponent::plain(tags.len().to_string()),
                format_list(&tags),
            ])
            .component(),
        _ => translations::COMMANDS_TAG_LIST_MULTIPLE_SUCCESS
            .message([
                TextComponent::plain(tagged.len().to_string()),
                TextComponent::plain(tags.len().to_string()),
                format_list(&tags),
            ])
            .component(),
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
