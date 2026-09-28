// Job DSL: the three pipelines, each reading its script from the repo.
def repo = System.getenv('REPO_URL') ?: 'https://github.com/nickmessing/automation.git'
def defaultBranch = System.getenv('REPO_BRANCH') ?: 'main'

multibranchPipelineJob('rates') {
    displayName('rates: build and test')
    description('Builds, lints and tests lab04-05/app on every branch that has lab04-05/pipelines/build_and_test.groovy.')
    branchSources {
        branchSource {
            source {
                git {
                    id('automation')
                    remote(repo)
                    traits {
                        gitBranchDiscovery()
                    }
                }
            }
        }
    }
    factory {
        workflowBranchProjectFactory {
            scriptPath('lab04-05/pipelines/build_and_test.groovy')
        }
    }
    triggers {
        // localhost cannot receive GitHub webhooks, so poll instead
        periodicFolderTrigger {
            interval('5m')
        }
    }
    orphanedItemStrategy {
        discardOldItems {
            numToKeep(5)
        }
    }
}

pipelineJob('ansible-setup') {
    displayName('test-server: configure with ansible')
    description('Runs lab04-05/ansible/setup_test_server.yml against test-server.')
    parameters {
        stringParam('BRANCH', defaultBranch, 'Branch of the repo to take the playbook from')
    }
    definition {
        cpsScm {
            scm {
                git {
                    remote { url(repo) }
                    branch('${BRANCH}')
                }
            }
            scriptPath('lab04-05/pipelines/ansible_setup.groovy')
            lightweight(false)
        }
    }
}

pipelineJob('deploy') {
    displayName('test-server: deploy rates')
    description('Copies a rates binary built by the rates job to test-server and restarts it.')
    parameters {
        stringParam('BRANCH', defaultBranch, 'Branch of the repo to take the playbooks from')
        stringParam('SOURCE_JOB', "rates/${defaultBranch}", 'Build job to take the binary from')
        stringParam('SOURCE_BUILD', '', 'Build number to deploy, empty for the last successful one')
    }
    definition {
        cpsScm {
            scm {
                git {
                    remote { url(repo) }
                    branch('${BRANCH}')
                }
            }
            scriptPath('lab04-05/pipelines/deploy.groovy')
            lightweight(false)
        }
    }
}
