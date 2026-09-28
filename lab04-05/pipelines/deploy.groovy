// Deploy a rates binary from the build job to test-server (lab05's php_deploy_pipeline).
pipeline {
    agent {
        label 'ansible-agent'
    }

    options {
        skipDefaultCheckout()
        timestamps()
        ansiColor('xterm')
        disableConcurrentBuilds()
    }

    environment {
        ANSIBLE_FORCE_COLOR = 'true'
    }

    stages {
        stage('Checkout') {
            steps {
                checkout scm
            }
        }

        stage('Fetch Build') {
            steps {
                copyArtifacts(
                    projectName: params.SOURCE_JOB,
                    selector: params.SOURCE_BUILD ? specific(params.SOURCE_BUILD) : lastSuccessful(),
                    filter: 'lab04-05/app/target/release/rates',
                    target: 'build',
                    flatten: true,
                    fingerprintArtifacts: true,
                )
                sh 'chmod +x build/rates && build/rates --version'
            }
        }

        stage('Deploy') {
            steps {
                dir('lab04-05/ansible') {
                    // deploy.yml also runs setup_test_server.yml, so a fresh server works too
                    sshagent(credentials: ['test-server-key']) {
                        sh 'ansible-playbook deploy.yml -e rates_binary="$WORKSPACE/build/rates"'
                    }
                }
            }
        }

        stage('Smoke Test') {
            steps {
                sh 'curl -fsS http://test-server/health'
                sh 'curl -fsS http://test-server/api/rate/EUR/USD'
                sh 'curl -fsS -o /dev/null -w "page: HTTP %{http_code}, %{size_download} bytes\\n" http://test-server/'
            }
        }
    }

    post {
        success {
            echo "Deployed ${params.SOURCE_JOB} #${params.SOURCE_BUILD ?: 'last successful'}: http://localhost:8081/"
        }
        failure {
            echo 'Deploy failed.'
        }
    }
}
