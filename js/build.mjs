import {build} from 'esbuild';
import {polyfillNode} from 'esbuild-plugin-polyfill-node';
import {join} from 'path';

const dir = process.argv[2];

build({
    entryPoints: [join(dir, 'src/index.ts')],
    bundle: true,
    outfile: join(dir, 'index.js'),
    plugins: [
        {
            name: 'polyfill',
            setup(build) {
                build.initialOptions.inject = [
                    join(process.cwd(), "poly.js"),
                    join(process.cwd(), "00_webidl.js"),
                    join(process.cwd(), "00_url.js"),
                ];
            }
        },
        polyfillNode({
            globals: {
                process: false,
                buffer: false,
            },
            polyfills: {
                "browser": false,
                "process": false,
            },
        }),
    ],
});
