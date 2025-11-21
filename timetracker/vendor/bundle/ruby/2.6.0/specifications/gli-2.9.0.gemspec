# -*- encoding: utf-8 -*-
# stub: gli 2.9.0 ruby lib

Gem::Specification.new do |s|
  s.name = "gli".freeze
  s.version = "2.9.0"

  s.required_rubygems_version = Gem::Requirement.new(">= 0".freeze) if s.respond_to? :required_rubygems_version=
  s.require_paths = ["lib".freeze]
  s.authors = ["David Copeland".freeze]
  s.date = "2014-01-19"
  s.description = "Build command-suite CLI apps that are awesome.  Bootstrap your app, add commands, options and documentation while maintaining a well-tested idiomatic command-line app".freeze
  s.email = "davidcopeland@naildrivin5.com".freeze
  s.executables = ["gli".freeze]
  s.extra_rdoc_files = ["README.rdoc".freeze, "gli.rdoc".freeze]
  s.files = ["README.rdoc".freeze, "bin/gli".freeze, "gli.rdoc".freeze]
  s.homepage = "http://davetron5000.github.com/gli".freeze
  s.rdoc_options = ["--title".freeze, "Git Like Interface".freeze, "--main".freeze, "README.rdoc".freeze]
  s.rubygems_version = "3.0.3.1".freeze
  s.summary = "Build command-suite CLI apps that are awesome.".freeze

  s.installed_by_version = "3.0.3.1" if s.respond_to? :installed_by_version

  if s.respond_to? :specification_version then
    s.specification_version = 4

    if Gem::Version.new(Gem::VERSION) >= Gem::Version.new('1.2.0') then
      s.add_development_dependency(%q<rake>.freeze, ["~> 0.9.2.2"])
      s.add_development_dependency(%q<rdoc>.freeze, ["~> 3.11"])
      s.add_development_dependency(%q<rainbow>.freeze, ["~> 1.1.1"])
      s.add_development_dependency(%q<clean_test>.freeze, [">= 0"])
      s.add_development_dependency(%q<cucumber>.freeze, ["= 1.2.3"])
      s.add_development_dependency(%q<gherkin>.freeze, ["<= 2.11.6"])
      s.add_development_dependency(%q<aruba>.freeze, ["= 0.5.1"])
      s.add_development_dependency(%q<sdoc>.freeze, [">= 0"])
      s.add_development_dependency(%q<faker>.freeze, ["= 1.0.0"])
    else
      s.add_dependency(%q<rake>.freeze, ["~> 0.9.2.2"])
      s.add_dependency(%q<rdoc>.freeze, ["~> 3.11"])
      s.add_dependency(%q<rainbow>.freeze, ["~> 1.1.1"])
      s.add_dependency(%q<clean_test>.freeze, [">= 0"])
      s.add_dependency(%q<cucumber>.freeze, ["= 1.2.3"])
      s.add_dependency(%q<gherkin>.freeze, ["<= 2.11.6"])
      s.add_dependency(%q<aruba>.freeze, ["= 0.5.1"])
      s.add_dependency(%q<sdoc>.freeze, [">= 0"])
      s.add_dependency(%q<faker>.freeze, ["= 1.0.0"])
    end
  else
    s.add_dependency(%q<rake>.freeze, ["~> 0.9.2.2"])
    s.add_dependency(%q<rdoc>.freeze, ["~> 3.11"])
    s.add_dependency(%q<rainbow>.freeze, ["~> 1.1.1"])
    s.add_dependency(%q<clean_test>.freeze, [">= 0"])
    s.add_dependency(%q<cucumber>.freeze, ["= 1.2.3"])
    s.add_dependency(%q<gherkin>.freeze, ["<= 2.11.6"])
    s.add_dependency(%q<aruba>.freeze, ["= 0.5.1"])
    s.add_dependency(%q<sdoc>.freeze, [">= 0"])
    s.add_dependency(%q<faker>.freeze, ["= 1.0.0"])
  end
end
