program detect_compiler
    use iso_fortran_env, only: compiler_version, compiler_options
    implicit none
#if defined(__flang__)
    print *, "Flang detected"
#elif defined(__LFORTRAN__)
    print *, "LFortran detected"
#elif defined(__INTEL_COMPILER)
    #ifdef __INTEL_COMPILER_CLASSIC
        print *, "Intel Fortran Compiler (ifort) detected"
    #else
        print *, "Intel Fortran Compiler (ifx) detected"
    #endif
#elif defined(__GFORTRAN__)
    print *, "GNU Fortran Compiler detected"
#elif defined(__PGI)
    print *, "PGI Fortran Compiler detected"
#elif defined(_CRAYFTN)
    print *, "Cray Fortran Compiler detected"
#elif defined(__IBM_COMPILER)
    print *, "IBM XL Fortran Compiler detected"
#elif defined(__SUNPRO_F90)
    print *, "Oracle Solaris Studio Fortran Compiler detected"
#elif defined(__NAG__)
    print *, "NAG Fortran Compiler detected"
#else
    print *, "Unknown Fortran Compiler detected"
#endif
    ! https://fortranwiki.org/fortran/show/compiler_options
    print '(4a)', 'This file was compiled by ', &
              compiler_version(), ' using the options ', &
              compiler_options()

end program detect_compiler