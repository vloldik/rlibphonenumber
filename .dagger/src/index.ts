import {
    dag,
    Container,
    Directory,
    object,
    func,
    argument,
} from "@dagger.io/dagger"
import { JAR, JDK_IMAGE, LOCK_FILE, RE2_DEFAULT, RUST_IMAGE, RUST_MODULE_CONTENT } from "./constants"

@object()
export class Rlibphonenumber {

    buildLibphonenumber(
        version: string,
        re2Version: string | null = RE2_DEFAULT,
        useRe2 = true,
    ): Container {
        const re2 = (text: string, orElse: string = '') => useRe2 ? text : orElse
        return this.buildBase()
            .withWorkdir("/tmp")
            .withExec(["git", "clone", "--depth", "1", "--branch", version,
                "https://github.com/google/libphonenumber.git",
            ])
            .withWorkdir("/tmp/libphonenumber/cpp")
            .withExec(["bash", "-euo", "pipefail", "-c", `
      curl -sSL https://github.com/google/re2/archive/refs/tags/${re2Version}.tar.gz | tar -xz
      cd re2-${re2Version}
      mkdir build && cd build
      cmake \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
        -DCMAKE_INSTALL_PREFIX=/opt/re2 \
        -DCMAKE_INSTALL_LIBDIR=lib \
        ..
      make -j$(nproc) && make install
      cd ../.. && rm -rf re2-${re2Version}
    `])
            .withExec(["sed", "-i",
                "s/\\bStringPiece\\b/re2::StringPiece/g",
                "src/phonenumbers/regexp_adapter_re2.cc",
            ])
            .withExec(["bash", "-euo", "pipefail", "-c", `
      cmake \
        -DCMAKE_BUILD_TYPE=Release \
        -DCMAKE_CXX_FLAGS="-O3 -DNDEBUG" \
        -DCMAKE_CXX_STANDARD=17 \
        -DUSE_BOOST=OFF -DUSE_RE2=${re2('ON', 'OFF')} -DUSE_ICU_REGEXP=${re2('OFF', 'ON')} \
        -DCMAKE_INSTALL_PREFIX=/opt/libphonenumber \
        -DCMAKE_INSTALL_LIBDIR=lib \
        ${re2('-DCMAKE_PREFIX_PATH=/opt/re2')} \
        .
      make -j$(nproc) && make install
    `])
    }

    @func()
    async generate(
        @argument({ defaultPath: '/' }) source: Directory,
        tag: string = "",
    ): Promise<Directory> {
        const resolvedTag = tag ? await this.resolveTag(tag) : await this.readLock(source)
        const src = tag ? await this.withFreshResources(source, resolvedTag) : source

        const generated = this.metadataRunner(src)

        return source
            .withDirectory(
                "crates/rlibphonenumber/src/generated",
                generated.directory("/project/crates/rlibphonenumber/src/generated"),
            )

            .withDirectory(
                "crates/rlibphonenumber/resources",
                generated.directory("/project/crates/rlibphonenumber/resources"),
            )
            .withDirectory(
                "resources",
                generated.directory("/project/resources")
            )
            .withDirectory(
                "crates/rlibphonenumber_macro/resources",
                generated.directory("/project/crates/rlibphonenumber_macro/resources"),
            )
            .withNewFile(LOCK_FILE, resolvedTag + "\n")
    }

    @func({ cache: 'never' })
    async fuzz(
        variant: string,
        @argument({ defaultPath: '/' }) source: Directory,
        maxTotalTime: number = 60,
        re2Version: string = RE2_DEFAULT,
        useRe2: boolean = true,
    ): Promise<string> {
        const version = await this.readLock(source)
        const built = this.buildLibphonenumber(version, re2Version, useRe2)

        const cargoRegistry = dag.cacheVolume("cargo-registry")
        const cargoGit = dag.cacheVolume("cargo-git")
        const cargoTarget = dag.cacheVolume(`cargo-target-nightly-fuzz`)

        const fuzzArgs = maxTotalTime > 0
            ? ["cargo", "+nightly", "fuzz", "run", variant, "--", `-max_total_time=${maxTotalTime}`]
            : ["cargo", "+nightly", "fuzz", "run", variant]

        const baseWithRust = this.buildBase()
            .withExec(["rustup", "toolchain", "install", "nightly", "--no-self-update"])
            .withExec(["cargo", "install", "cargo-fuzz", "--locked"])

        return this.withPhoneLibs(baseWithRust, built)
            .withEnvVariable("DEBIAN_FRONTEND", "noninteractive")
            .withDirectory("/usr/local/include", built.directory("/opt/libphonenumber/include"))
            .withDirectory("/usr/local/lib", built.directory("/opt/libphonenumber/lib"))
            .withExec(["ldconfig"])
            .withMountedCache("/usr/local/cargo/registry", cargoRegistry)
            .withMountedCache("/usr/local/cargo/git", cargoGit)
            .withMountedDirectory("/project", source)
            .withMountedCache("/project/fuzz/target", cargoTarget)
            .withWorkdir("/project")
            .withExec(fuzzArgs)
            .stdout()
    }

    @func()
    async bumpVersion(
        @argument() source: Directory,
        tag: string,
    ): Promise<Directory> {
        const cargoRegistry = dag.cacheVolume("cargo-registry")

        const ctr = dag.container()
            .from(RUST_IMAGE)
            .withExec(["apt-get", "update", "-qq"])
            .withExec(["apt-get", "install", "-y", "--no-install-recommends", "jq"])
            .withMountedCache("/usr/local/cargo/registry", cargoRegistry)
            .withMountedDirectory("/project", source)
            .withWorkdir("/project")
            .withExec(["cargo", "install", "cargo-edit", "--locked"])
            .withExec(["cargo", "set-version", "--bump", "patch",
                "-p", "rlibphonenumber",
                "-p", "rlibphonenumber_macro",
            ])

        const newVersion = (await ctr
            .withExec(["bash", "-c",
                "cargo metadata --format-version 1 --no-deps | jq -r '.packages[0].version'",
            ])
            .stdout()).trim()

        return ctr
            .withExec(["sed", "-E", "-i",
                `s/rlibphonenumber = "[0-9]+\\.[0-9]+\\.[0-9]+"/rlibphonenumber = "${newVersion}"/g`,
                "Readme.md",
            ])
            .withExec(["sed", "-E", "-i",
                `s/Used metadata version: v[0-9]+\\.[0-9]+\\.[0-9]+/Used metadata version: ${tag}/g`,
                "Readme.md",
            ])
            .directory("/project")
    }

    private buildBase(): Container {
        return dag.container()
            .from(RUST_IMAGE)
            .withEnvVariable("DEBIAN_FRONTEND", "noninteractive")
            .withExec(["apt-get", "update", "-qq"])
            .withExec(["apt-get", "install", "-y", "--no-install-recommends",
                "build-essential", "cmake", "git", "curl", "ca-certificates",
                "pkg-config", "libssl-dev",
                "libprotobuf-dev", "protobuf-compiler",
                "libicu-dev", "libabsl-dev", "libgtest-dev",
                "llvm", "clang"
            ])
    }

    private withPhoneLibs(ctr: Container, built: Container): Container {
        return ctr
            .withDirectory("/opt/libphonenumber", built.directory("/opt/libphonenumber"))
            .withDirectory("/opt/re2", built.directory("/opt/re2"))
    }

    private async readLock(source: Directory): Promise<string> {
        return (await source.file(LOCK_FILE).contents()).trim()
    }

    private async resolveTag(tag: string): Promise<string> {
        if (tag !== "latest-supported") return tag

        const refs = await dag.container()
            .from("alpine/git")
            .withExec(["git", "ls-remote", "--tags",
                "https://github.com/google/libphonenumber.git",
            ])
            .stdout()

        const versions = [...refs.matchAll(/refs\/tags\/(v9\.0\.(\d+))/g)]
            .map(m => ({ tag: m[1], patch: parseInt(m[2]) }))
            .sort((a, b) => a.patch - b.patch)

        const latest = versions.at(-1)
        if (!latest) throw new Error("No tags v9.0.x found")
        return latest.tag
    }

    private async withFreshResources(
        source: Directory,
        tag: string,
    ): Promise<Directory> {
        const resources = dag.container()
            .from("alpine/git")
            .withExec(["git", "clone", "--depth", "1", "--branch", tag,
                "https://github.com/google/libphonenumber.git", "/libphonenumber",
            ])
            .directory("/libphonenumber/resources")

        return source.withDirectory("resources", resources)
    }

    private metadataRunner(source: Directory): Container {
        return this.buildBase()
            .withMountedDirectory("/project", source)
            .withWorkdir("/project")
            .withExec(this.callCliGenerate(
                "resources/PhoneNumberMetadata.xml",
                "crates/rlibphonenumber/src/generated/metadata",
                "metadata", "METADATA",
            ))
            .withExec(this.callCliGenerate(
                "resources/PhoneNumberMetadataForTesting.xml",
                "crates/rlibphonenumber/src/generated/metadata",
                "test_metadata", "TEST_METADATA",
            ))
            .withExec(this.callCliGenerate(
                "resources/PhoneNumberAlternateFormats.xml",
                "crates/rlibphonenumber/src/generated/metadata",
                "alternate_formats", "ALTERNATE_FORMATS_METADATA",
                /* validate as alternate formats */true
            ))
            .withNewFile(
                "crates/rlibphonenumber/src/generated/metadata/mod.rs",
                RUST_MODULE_CONTENT,
            )
            .withExec(["bash", "-c", "cp resources/*.proto crates/rlibphonenumber/resources/"])
            .withExec(["bash", "-c",
                "cp resources/PhoneNumberMetadata.xml crates/rlibphonenumber_macro/resources/",
            ])
    }

    private callCliGenerate(
        input: string, output: string, name: string, constName: string, alternateFormats?: boolean,
    ): string[] {
        let args = [
            "cargo", "run", "-p", "rlibphonenumber_cli",
            "build-metadata",
            input, output, name,
            "--const-name", constName,
            '-m'
        ]
        if (alternateFormats) {
            args.push('--alternate-formats')
        }

        return args
    }
}
