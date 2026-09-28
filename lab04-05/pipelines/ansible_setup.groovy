// Configure test-server with ansible (lab05's ansible_setup_pipeline).
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

        stage('Check Playbook') {
            steps {
                dir('lab04-05/ansible') {
                    sh 'ansible --version'
                    sh 'ansible-playbook --syntax-check setup_test_server.yml'
                }
            }
        }

        stage('Configure Test Server') {
            steps {
                dir('lab04-05/ansible') {
                    sshagent(credentials: ['test-server-key']) {
                        sh 'ansible-playbook setup_test_server.yml'
                    }
                }
            }
        }
    }

    post {
        success {
            echo 'test-server is configured.'
        }
        failure {
            echo 'Configuring test-server failed.'
        }
    }
}
