# -*- mode: ruby -*-
# vi: set ft=ruby :

Vagrant.configure("2") do |config|
  config.vm.synced_folder ".", "/home/vagrant/swell_history"
  config.vm.hostname = "swell-history"
  config.vm.network :private_network, ip: "192.168.33.8"

  config.vm.provider "virtualbox" do |vb, override|
    override.vm.box = "ubuntu/focal64"
    vb.customize ["modifyvm", :id, "--memory", "4096"]
    vb.customize ["modifyvm", :id, "--cpus", "2"]
    vb.customize ["modifyvm", :id, "--uartmode1", "disconnected"]
  end

  # Keep roles from getting installed everytime -> https://stackoverflow.com/a/46045193
  config.vm.provision 'preemptively give others write access to /etc/ansible/roles', type: :shell, inline: <<~'EOM'
    sudo mkdir /etc/ansible/roles -p
    sudo chmod o+w /etc/ansible/roles
  EOM

  config.vm.provision "ansible_local" do |ansible|
    ansible.compatibility_mode = "2.0"
    ansible.galaxy_role_file = '/home/vagrant/swell_history/requirements.yml'
    ansible.playbook = "/home/vagrant/swell_history/main.yml"
    ansible.galaxy_roles_path = '/etc/ansible/roles'
    ansible.galaxy_command = 'ansible-galaxy install --role-file=%{role_file} --roles-path=%{roles_path}'
  end
  # start in the project working directory
  config.ssh.extra_args = ["-t", "cd /home/vagrant/swell_history; bash --login"]
end
