import {parse} from 'graphql/language/parser';
import {composeAndValidate} from '@apollo/federation';

interface Input {
    services: Array<{ name: string; sdl: string; url?: string }>,
}

interface Output {
    sdl?: string,
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

globalThis.compose = function compose(
    input: Input
): Output {
    try {
        const definitions = input.services.map(({name, sdl, url}) => ({
            name,
            url,
            typeDefs: parse(sdl),
        }));

        const result = composeAndValidate(definitions);
        const errors = result.errors ? result.errors.map(rewriteError) : undefined;
        return {sdl: result.supergraphSdl, errors};
    } catch (e) {
        if (e instanceof Error) {
            return {errors: [{message: e.message}]};
        } else {
            return {errors: [{message: 'non-error thrown'}]};
        }
    }
}
