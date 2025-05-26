# nabs: Not a Build System

**NOTE:** This project is not in a very useful state right now, I'm only planning to use it personal projects to see if its any good.  

`nabs` models your monorepo as a graph and finds all the affected packages for a given changeset.  

Monorepos allow you to change all the code in a single PR, this has great benefits for developer velocity. In the beginning, its fine to run all the tests in the repo in a single pipeline. Once your packages start to grow, your CI time balloons up. In this case, it would make sense to have a single pipeline for every package in the monorepo.  
The problem now is, detecting which pipelines to run. A common workflow is this:
- find the diff in the PR using git
- find all the affected packages and services using this diff **(`nabs` does this)**
- run pipelines for all the affected packages

This can be done in `bazel` and friends, for example, using `bazel query 'rdeps(//my-target)'`. All the build systems with monorepos give this feature.   
These build tools come with their own set of pains though:
- steep learning curve
- high maintenance cost
- bad IDE integration
- huge upfront investment in building a custom CI

Tools like `bazel` are extremely feature-rich, fast and provide some amazing qualities (like hermeticity). They are however, built for scale, where you have a dedicated engineering team taking care of it. They give the fast monorepo experience at scale.  
`nabs` is not meant for those use-cases. `nabs` is for mid-sized engineering teams, who just want a simple monorepo setup while using existing tooling and infrastructure.  

The only explicit goal of `nabs` is to track dependencies, it is neither a build executor, nor a test runner, nor a remote execution framework. It's just a simple package tracker.  
An explicit goal for `nabs` is to work with current tooling. In your monorepo, you could add an empty `nabs.json` file and `nabs` would start tracking it as a package. If you have `requirements.txt` in that package, `nabs` would automatically start treating it as a python package, find all the local dependencies in your workspace and start tracking them recursively.  

# Getting started

Add an empty `workspace.json` in the root of your monorepo.  
```sh
echo "{}" > workspace.json
```

In every package in the monorepo, add an empty `nabs.json` file
```sh
echo "{}" > nabs.json
```
`nabs` would start tracking your package. Do this for every package in the monorepo.  


You can take a look at the graph `nabs` creates using

```sh
# from any folder inside the workspace
nabs graph
```

`nabs changeset` takes a list of files as input, finds the packages those files belong to, and finds all the packages which transitively depend on these packages. You can now run only the tests for affected packages in a PR (or your main branch build, you need to find a way to find the diff from the last successful build from your CI provider).  

A simple script for this
```sh
GIT_CHANGES=$(git diff-tree --no-commit-id --name-only -r origin/main my-awesome-branch)
AFFECTED_PACKAGES=$(echo $GIT_CHANGES | nabs changeset)

# run a script inside the package directory
# or you could run a github action, or a jenkins pipeline
echo $AFFECTED_PACKAGES | while read pkg_dir; do $pkg_dir/run_test.sh; done
```

## Supported build systems/languages
| language | tool/manifest file |
|----------|--------------------|
|  python  | requirements.txt |
| rust | Cargo.toml |

## should I use `nabs`?

- `nabs` is supposed to be useful in that spot where your monorepo has started taking a lot of your CI time but you don't want to do a big investment in a stronger build tool.    
- you should not use `nabs` if you have huge repositories with many developers, in this case, `bazel` and other build tools are clear winners.  