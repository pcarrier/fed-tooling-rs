import {parse} from 'graphql/language/parser';
import {composeServices} from '@apollo/composition';
import {CompositionHint} from '@apollo/composition/dist/hints';

interface Input {
    services: Array<{ name: string; sdl: string; url?: string }>,
}

interface Output {
    sdl?: string,
    hints?: readonly CompositionHint[]
    errors: readonly Error[]
}

function rewriteNode(node) {
    let { kind, name, subgraph, loc } = node;
    loc = loc ? [loc.start, loc.end] : undefined;
    name = name ? name.value : undefined;
    return { kind, name, subgraph, loc };
}

function rewriteError(error) {
    const errors = error.errors ? error.errors.map(rewriteError) : undefined;
    const nodes = error.nodes ? error.nodes.map(rewriteNode) : undefined;
    return {...error, errors, nodes, source: undefined, stack: undefined};
}

function rewriteHint(hint) {
    const nodes = hint.nodes ? hint.nodes.map(rewriteNode) : undefined;
    return {...hint, nodes};
}

globalThis.compose = function compose(
    input: Input
): Output {
    try {
        const definitions = input.services.map(({name, sdl, url}) => ({
            name,
            url,
            typeDefs: parse(sdl),
        }));

        const result = composeServices(definitions);
        const errors = result.errors ? result.errors.map(rewriteError) : undefined;
        const hints = result.hints ? result.hints.map(rewriteHint) : undefined;
        return {sdl: result.supergraphSdl, errors, hints};
    } catch (e) {
        if (e instanceof Error) {
            return {errors: [{message: e.message}]};
        } else {
            return {errors: [{message: 'non-error thrown'}]};
        }
    }
}
