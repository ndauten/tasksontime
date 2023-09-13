# Ensure we require the local version and not one we might have installed already
require File.join([File.dirname(__FILE__),'lib','timetracker','version.rb'])
spec = Gem::Specification.new do |s| 
  s.name = 'timetracker'
  s.version = Timetracker::VERSION
  s.author = 'Nathan Dautenhahn'
  s.email = 'nathan.dautenhahn@gmail.com'
  s.homepage = 'https://nathandautenhahn.com'
  s.platform = Gem::Platform::RUBY
  s.summary = 'Track time for the motivation'
  s.files = `git ls-files`.split("
")
  s.require_paths << 'lib'
  s.extra_rdoc_files = ['README.rdoc','timetracker.rdoc']
  s.rdoc_options << '--title' << 'timetracker' << '--main' << 'README.rdoc' << '-ri'
  s.bindir = 'bin'
  s.executables << 'timetracker'
  s.add_development_dependency('rake')
  s.add_development_dependency('rdoc')
  s.add_runtime_dependency('gli','2.9.0')
end
