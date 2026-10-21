"""Prepare a QA-only Surface native-build supplement; preserve frozen sources."""
from pathlib import Path
import difflib
import hashlib
import json
import re
import subprocess

OUT = Path(__file__).resolve().parent
STAGE = Path('/private/tmp/osv-sweep-common-source-v11-20261018')
NODE = '/opt/homebrew/Cellar/node@22/22.23.3_3/bin/node'


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    original = STAGE / 'scripts/check-sweep-miter-matrix-browser.mjs'
    raw = original.read_text()
    original_hash = sha(original.read_bytes())
    assert original_hash == 'd3e2e35d3ec86af6ba2f921b7c0fa848748f19170bdd0779f507dd4d06559a87'
    runner = OUT / 'check-v11-surface-native-build-matrix-20261018.mjs'
    assert not runner.exists()
    text = raw
    patches = []

    def replace(before, after, label):
        nonlocal text
        assert text.count(before) == 1, (label, text.count(before))
        text = text.replace(before, after)
        patches.append(label)

    replace("const surfaceMode=process.argv.includes('--surface')",
            "const surfaceMode=process.argv.includes('--surface')\nconst surfaceNativeBuildOnly=process.argv.includes('--surface-native-build-only')",
            'explicit Surface-only flag')
    replace("assert.ok(!evidenceOnly||surfaceMode&&!headed,'Evidence-only requires a headless surface run')",
            "assert.ok(!evidenceOnly||surfaceMode&&!headed,'Evidence-only requires a headless surface run')\nassert.ok(surfaceNativeBuildOnly&&surfaceMode&&nativeRunningCancel&&!headed&&!evidenceOnly,'Supplement requires actual running native Surface builds and headless full rendering')",
            'require actual native/headless/rendering criteria')
    replace("assert.ok(!surfaceMode,'Native cancellation probe requires a retained body source')",
            "assert.ok(!surfaceMode||surfaceNativeBuildOnly,'Native Surface builds require the explicit supplement')",
            'enable authenticated native build probes for Surface')
    replace('const busySource=progressiveStationMatrixMode?',
            "const busySource=surfaceNativeBuildOnly?source+'\\n// Observe original native surface work in a new revision.\\n':progressiveStationMatrixMode?",
            'preserve original Surface geometry and all budgets')
    replace("     entry.assertions.push({case:buildAction==='cancel'?'native-build-cancellation':'native-build-source-change'",
            "     if(surfaceNativeBuildOnly)assert.equal(await page.getByTestId('sweep-patch-final-evidence').count(),0,'Native surface cancellation must clear final patch certificate presentation')\n     entry.assertions.push({case:buildAction==='cancel'?'native-build-cancellation':'native-build-source-change'",
            'require actual Surface certificate clearing')
    replace("    for(const action of ['cancel','source-change']){",
            "    if(!surfaceNativeBuildOnly)for(const action of ['cancel','source-change']){",
            'exclude native Solid operation for surfaces; preserve existing Solid refusal tests')
    replace('report.browser=browser.version()',
            "report.supplementalHarness={kind:'surface-native-build-only',runnerSha256:sha(await readFile(new URL(import.meta.url))),originalRunnerSha256:'" + original_hash + "',scope:'Native Surface build cancellation and source replacement only; native Solid is inapplicable and existing Surface Solid refusal controls remain.'}\nreport.browser=browser.version()",
            'record exact additive QA harness identity/scope')
    imports = []

    def import_path(match):
        path = STAGE / 'scripts' / match.group(1)
        assert path.is_file()
        imports.append({'file': str(path), 'sha256': sha(path.read_bytes())})
        return "from '" + path.as_uri() + "'"

    text = re.sub(r"from '\./([^']+)'", import_path, text)
    assert len(imports) == 8
    archive = OUT / 'surface-native-original-browser-checker-20261018.mjs'
    assert not archive.exists()
    runner.write_text(text)
    archive.write_text(raw)
    patch = OUT / 'surface-native-build-only-harness-20261018.patch'
    patch.write_text(''.join(difflib.unified_diff(raw.splitlines(True), text.splitlines(True),
                                               fromfile=str(original), tofile=str(runner))))
    subprocess.run([NODE, '--check', str(runner)], check=True)
    assert sha(original.read_bytes()) == original_hash
    report = {'schema': 'sweep-v11-surface-native-harness/1', 'state': 'prepared-runtime-unqualified',
              'originalRunnerSha256': original_hash, 'runnerSha256': sha(runner.read_bytes()),
              'runner': str(runner), 'preparerSha256': sha(Path(__file__).resolve().read_bytes()),
              'patchSha256': sha(patch.read_bytes()), 'unchangedImports': imports, 'changes': patches,
              'sourceGeometryAndBudgetsUnchanged': True, 'nativeObservationCriteriaUnchanged': True,
              'surfaceBuildObservationsRequired': 220, 'nativeSolidApplicable': False, 'fullGoalComplete': False}
    (OUT / 'v11-surface-native-harness-20261018.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps({'prepared': True, 'unchangedOriginal': True, 'changes': patches,
                      'imports': len(imports), 'syntaxPassed': True, 'runtimeQualified': False}, indent=2))


if __name__ == '__main__':
    main()
