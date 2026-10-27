import { plugin } from 'bun';
import { readFileSync } from 'node:fs';
import { compileScript, parse } from '@vue/compiler-sfc';

// Compile the real small Vue components, rather than a test-only renderer.
plugin({
  name: 'vue-security-tests',
  setup(build) {
    build.onLoad({ filter: /\.vue$/ }, ({ path }) => {
      const { descriptor, errors } = parse(readFileSync(path, 'utf8'), { filename: path });
      if (errors.length) throw errors[0];
      return {
        contents: compileScript(descriptor, { id: path, inlineTemplate: true }).content,
        loader: 'ts',
      };
    });
  },
});
