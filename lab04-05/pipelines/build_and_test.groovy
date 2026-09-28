// Build, lint and test the Rust app (lab04's Jenkinsfile, lab05's php_build_and_test_pipeline).
pipeline {
    agent {
        label 'rust-agent'
    }

    options {
        skipDefaultCheckout()
        timestamps()
        ansiColor('xterm')
        buildDiscarder(logRotator(numToKeepStr: '20'))
        // the deploy job copies the release binary from here
        copyArtifactPermission('/deploy')
    }

    environment {
        CARGO_TERM_COLOR = 'always'
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
            }
        }

        stage('Install Dependencies') {
            steps {
                dir('lab04-05/app') {
                    sh 'cargo --version && cargo nextest --version'
                    sh 'cargo fetch --locked'
                }
            }
        }

        stage('Lint') {
            steps {
                dir('lab04-05/app') {
                    sh 'cargo fmt --check'
                    sh 'cargo clippy --locked --all-targets -- -D warnings'
                }
            }
        }

        stage('Test') {
            steps {
                dir('lab04-05/app') {
                    sh 'cargo nextest run --locked --profile ci'
                }
            }
            post {
                always {
                    junit testResults: 'lab04-05/app/target/nextest/ci/junit.xml', allowEmptyResults: false
                }
            }
        }

        stage('Release Build') {
            steps {
                dir('lab04-05/app') {
                    sh 'cargo build --locked --release'
                    sh './target/release/rates --version'
                }
                archiveArtifacts artifacts: 'lab04-05/app/target/release/rates', fingerprint: true
            }
        }

        stage('Deploy') {
            when {
                expression { env.BRANCH_NAME == env.DEPLOY_BRANCH }
            }
            steps {
                build job: '/deploy', wait: false, parameters: [
                    string(name: 'BRANCH', value: env.BRANCH_NAME),
                    string(name: 'SOURCE_JOB', value: env.JOB_NAME),
                    string(name: 'SOURCE_BUILD', value: env.BUILD_NUMBER),
                ]
            }
        }
    }

    post {
        always {
            echo 'Pipeline completed.'
        }
        success {
            echo 'All stages completed successfully!'
        }
        failure {
            echo 'Errors detected in the pipeline.'
        }
    }
}
